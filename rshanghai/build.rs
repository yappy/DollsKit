use anyhow::{bail, Result};
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

fn command_raw(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        warn_lines(&String::from_utf8_lossy(&output.stderr));
        if let Some(code) = output.status.code() {
            bail!("{program}: exit code = {code}");
        } else {
            bail!("{program}: Terminated by signal");
        }
    }
}

fn command(program: &str, args: &[&str], def: &[&str]) -> Vec<String> {
    match command_raw(program, args) {
        Ok(stdout) => stdout.lines().map(|s| s.to_string()).collect(),
        Err(err) => {
            warn(&format!("command error: {}", cmdline_str(program, args)));
            warn_lines(&err.to_string());
            def.iter().map(|s| s.to_string()).collect()
        }
    }
}

fn git_info() {
    // get relative path to .git/HEAD file
    let git_head_path = command(
        "git",
        &["rev-parse", "--git-path", "HEAD"],
        &[""],
    );
    // rerun if .git/HEAD is changed
    println!("cargo::rerun-if-changed={}", git_head_path[0]);

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
