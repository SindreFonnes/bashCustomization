use std::process::Command;

use anyhow::{Context, Result, bail};

/// Run a command, capture stdout, fail on non-zero exit.
pub fn run(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute: {program}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "{program} exited with {}: {}",
            output.status,
            stderr.trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a command inheriting stdin/stdout/stderr so the user sees output.
pub fn run_visible(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute: {program}"))?;

    if !status.success() {
        bail!("{program} exited with {status}");
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
