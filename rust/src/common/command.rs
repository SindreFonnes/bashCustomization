use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail};

static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Control whether subprocess output streams directly to the terminal.
pub fn set_verbose(verbose: bool) {
    VERBOSE.store(verbose, Ordering::Relaxed);
}

/// Run a command, capture stdout, fail on non-zero exit.
pub fn run(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute: {program}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{program} exited with {}: {}", output.status, stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a command, capturing output by default and inheriting it in verbose mode.
pub fn run_visible(program: &str, args: &[&str]) -> Result<()> {
    if VERBOSE.load(Ordering::Relaxed) {
        let status = Command::new(program)
            .args(args)
            .status()
            .with_context(|| format!("failed to execute: {program}"))?;

        if !status.success() {
            bail!("{program} exited with {status}");
        }

        return Ok(());
    }

    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute: {program}"))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "{program} exited with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            stdout.trim(),
            stderr.trim()
        );
    }

    Ok(())
}

/// Check if a command exists on PATH.
pub fn exists(program: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {program}")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if the current process is running as root.
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_failure_includes_stdout_and_stderr() {
        set_verbose(false);
        let error = run_visible(
            "sh",
            &[
                "-c",
                "printf 'captured-out'; printf 'captured-err' >&2; exit 7",
            ],
        )
        .expect_err("command should fail");
        let message = format!("{error:#}");

        assert!(message.contains("captured-out"));
        assert!(message.contains("captured-err"));
    }
}
