#!/usr/bin/env python3

"""Synchronize Beads issues to GitHub Issues.

This is intentionally usable from a checkout on a developer machine.  The
GitHub Actions workflow can later invoke the same command after installing
``bd`` and bootstrapping the Dolt database.

``bd github`` requires following configurations.
Configuration can be set via 'bd config' or environment variables:
  github.token / GITHUB_TOKEN           - Personal access token
  github.owner / GITHUB_OWNER           - Repository owner
"""

import argparse
import json
import subprocess
from dataclasses import dataclass
from typing import Any, Iterable
from urllib.parse import urlparse


MARKER = "<!-- beads-github-sync:v1 -->"
BODY_LIMIT = 65_000
RETRY_COUNT = 4


@dataclass(frozen=True)
class Options:
    dry_run: bool
    repo: str | None


def run(command: list[str]) -> str:
    try:
        result = subprocess.run(
            command,
            check=True,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except subprocess.CalledProcessError as error:
        detail = error.stderr.strip() or error.stdout.strip()
        raise RuntimeError(
            f"Command failed ({error.returncode}): "
            f"{' '.join(command)}\n{detail}"
        ) from error

    return result.stdout


def run_json(command: list[str]) -> Any:
    stdout = run(command)

    try:
        return json.loads(stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(
            f"Could not parse JSON: {' '.join(command)}"
        ) from error


def bead_issues() -> list[dict[str, Any]]:
    issues = run_json(["bd", "list", "--all", "--json"])
    if not isinstance(issues, list):
        raise RuntimeError("The result of bd list is not an array")
    return issues


def issue_labels(issues: Iterable[dict[str, Any]]) -> list[str]:
    return sorted(
        {
            label
            for issue in issues
            for label in issue.get("labels", [])
            if isinstance(label, str) and label
        }
    )


def gh_command(args: list[str], repo: str | None = None) -> list[str]:
    command = ["gh", *args]
    if repo:
        command.extend(["--repo", repo])
    return command


def prepare_labels(labels: Iterable[str], options: Options) -> None:
    for label in labels:
        command = gh_command(
            [
                "label",
                "create",
                label,
                "--color",
                "ededed",
                "--description",
                "Imported from Beads",
                "--force",
            ],
            options.repo,
        )
        if options.dry_run:
            print("DRY-RUN:", " ".join(command))
            continue
        run(command)


def sync_beads(options: Options) -> None:
    command = ["bd", "github", "sync", "--push-only"]
    if options.dry_run:
        command.append("--dry-run")
    # Run twice because bd 1.2.2 creates new Issues as open and applies the
    # closed state on a subsequent update.
    for pass_number in (1, 2):
        print(f"Beads -> GitHub sync ({pass_number}/2)")
        run(command)


def external_issue_url(issue: dict[str, Any]) -> str | None:
    value = issue.get("external_ref")
    if not isinstance(value, str) or not value:
        return None
    parsed = urlparse(value)
    if parsed.scheme != "https" or parsed.netloc != "github.com":
        raise RuntimeError(f"Not a GitHub Issue URL ({issue.get('id')}): {value}")
    if "/issues/" not in parsed.path:
        raise RuntimeError(f"Not a GitHub Issue URL ({issue.get('id')}): {value}")
    return value


def validate_external_refs(issues: Iterable[dict[str, Any]]) -> None:
    seen: dict[str, str] = {}
    for issue in issues:
        url = external_issue_url(issue)
        if not url:
            continue
        issue_id = value(issue, "id", default="unknown")
        previous = seen.get(url)
        if previous:
            raise RuntimeError(
                f"Duplicate external_ref: {url} ({previous}, {issue_id})"
            )
        seen[url] = issue_id


def comments_for(issue_id: str) -> list[dict[str, Any]]:
    comments = run_json(["bd", "comments", issue_id, "--json"])
    if not isinstance(comments, list):
        raise RuntimeError(f"The comments result is not an array: {issue_id}")
    return comments


def value(item: dict[str, Any], *keys: str, default: str = "") -> str:
    for key in keys:
        candidate = item.get(key)
        if candidate is not None and candidate != "":
            return str(candidate)
    return default


def section(title: str, content: str) -> str:
    return f"## {title}\n\n{content.strip() or '_None_'}\n"


def render_comments(comments: list[dict[str, Any]]) -> str:
    if not comments:
        return "_None_"
    rendered = []
    for comment in comments:
        author = value(comment, "author", "created_by", "user", default="unknown")
        timestamp = value(comment, "created_at", "updated_at", default="unknown time")
        body = value(comment, "body", "text", "content", "comment")
        rendered.append(f"### {author} ({timestamp})\n\n{body.strip()}")
    return "\n\n".join(rendered)


def render_body(issue: dict[str, Any], comments: list[dict[str, Any]]) -> str:
    issue_id = value(issue, "id", default="unknown")
    lines = [
        MARKER,
        "> This body is generated automatically from Beads.",
        "> Changes to synchronized content on GitHub will be overwritten on the next sync.",
        "",
        f"Beads issue: `{issue_id}`",
        "",
        section("Description", value(issue, "description")),
        section("Design", value(issue, "design")),
        section("Acceptance Criteria", value(issue, "acceptance_criteria")),
        section("Notes", value(issue, "notes")),
        section("Comments", render_comments(comments)),
    ]
    return "\n".join(lines).rstrip() + "\n"


def truncate_body(
    issue: dict[str, Any], body: str, comments: list[dict[str, Any]]
) -> str:
    if len(body) <= BODY_LIMIT:
        return body
    remaining = list(comments)
    while remaining and len(body) > BODY_LIMIT:
        remaining.pop(0)
        body = render_body(issue, remaining)
    if len(body) > BODY_LIMIT:
        suffix = (
            "\n\n_Long comments were omitted because the body exceeded GitHub's limit._\n"
            "See Beads for the complete content.\n"
        )
        body = body[: BODY_LIMIT - len(suffix)].rstrip() + suffix
    return body


def update_issue(issue: dict[str, Any], options: Options) -> None:
    url = external_issue_url(issue)
    if not url:
        print(f"SKIP {issue.get('id')}: no external_ref")
        return
    comments = comments_for(value(issue, "id"))
    body = truncate_body(issue, render_body(issue, comments), comments)
    current = run_json(
        gh_command(["issue", "view", url, "--json", "body"], options.repo)
    )
    old_body = current.get("body", "") if isinstance(current, dict) else ""
    if old_body == body:
        print(f"UNCHANGED {issue.get('id')}")
        return
    command = gh_command(["issue", "edit", url, "--body", body], options.repo)
    if options.dry_run:
        print(f"DRY-RUN UPDATE {issue.get('id')}: body {len(old_body)} -> {len(body)} characters")
    else:
        run(command)
        print(f"UPDATED {issue.get('id')}")


def parse_args() -> Options:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dry-run", action="store_true", help="show actions without changing anything")
    parser.add_argument("--repo", help="target repository for gh (OWNER/REPO)")
    args = parser.parse_args()

    return Options(args.dry_run, args.repo)


def main() -> int:
    options = parse_args()

    before = bead_issues()
    prepare_labels(issue_labels(before), options)
    sync_beads(options)
    issues = bead_issues()
    validate_external_refs(issues)
    for issue in issues:
        update_issue(issue, options)

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
