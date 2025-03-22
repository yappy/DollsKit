use anyhow::{Result, bail};
use std::process::Command;

// `cargo build -vv` to debug this.

fn warn(msg: &str) {
    println!("cargo::warning={msg}");
}

fn warn_lines(lines: &str) {
    for line in lines.lines() {
        warn(line);
    }
}

fn setenv(key: &str, value: &str) {
    if value.is_empty() {
        println!("cargo::rustc-env={key}=");
    } else {
        for v in value.lines().take(1) {
            println!("cargo::rustc-env={key}={v}");
        }
    }
}

fn cmdline_str(program: &str, args: &[&str]) -> String {
    let mut cmdline = program.to_string();
    for arg in args {
        cmdline.push(' ');
        cmdline.push_str(arg);
    }

    cmdline
}

fn command_raw(program: &str, args: &[&str], warn_on: bool) -> Result<String> {
    let output = Command::new(program).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        if warn_on {
            warn_lines(&String::from_utf8_lossy(&output.stderr));
        }
        if let Some(code) = output.status.code() {
            bail!("{program}: exit code = {code}");
        } else {
            bail!("{program}: Terminated by signal");
        }
    }
}

fn command_with_default(program: &str, args: &[&str], def: &[&str], warn_on: bool) -> Vec<String> {
    match command_raw(program, args, warn_on) {
        Ok(stdout) => stdout.lines().map(|s| s.to_string()).collect(),
        Err(err) => {
            if warn_on {
                warn(&format!("command error: {}", cmdline_str(program, args)));
                warn_lines(&err.to_string());
            }

            def.iter().map(|s| s.to_string()).collect()
        }
    }
}

fn command(program: &str, args: &[&str], def: &[&str]) -> Vec<String> {
    command_with_default(program, args, def, true)
}

fn command_no_warn(program: &str, args: &[&str], def: &[&str]) -> Vec<String> {
    command_with_default(program, args, def, false)
}

fn rerun_by_git_refs(name: &str) {
    // get relative path to the file of <name>, then add to rerun_if_changed
    let git_refs_path = command("git", &["rev-parse", "--git-path", name], &[""]);
    println!("cargo::rerun-if-changed={}", git_refs_path[0]);

    // if it is a symbolic ref, resolve link by 1.
    let next = command_no_warn("git", &["symbolic-ref", name], &[]);
    if !next.is_empty() {
        rerun_by_git_refs(&next[0]);
    }
}

fn git_info() {
    rerun_by_git_refs("HEAD");

    let val = command(
        "git",
        &["describe", "--always", "--dirty"],
        &["git-describe-unknown"],
    );
    setenv("BUILD_GIT_DESCRIBE", &val[0]);

    let val = command("git", &["symbolic-ref", "HEAD"], &["git-branch-unknown"]);
    setenv("BUILD_GIT_BRANCH", &val[0]);

    let val = command(
        "git",
        &["show", "HEAD", "--pretty=format:%h"],
        &["git-hash-unknown"],
    );
    setenv("BUILD_GIT_HASH", &val[0]);

    // author date
    // commiter date is "%cs"
    let val = command(
        "git",
        &["show", "HEAD", "--pretty=%as"],
        &["git-date-unknown"],
    );
    setenv("BUILD_GIT_DATE", &val[0]);
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    setenv("BUILD_CARGO_DEBUG", &std::env::var("DEBUG").unwrap());
    setenv("BUILD_CARGO_TARGET", &std::env::var("TARGET").unwrap());

    git_info();
}
