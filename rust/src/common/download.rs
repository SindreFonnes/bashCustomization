use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

fn client() -> Client {
    Client::builder()
        .user_agent("bashc/0.1.0")
        .build()
        .expect("failed to build HTTP client")
}

/// Download a URL to a file with a progress bar.
pub fn download_file(url: &str, dest: &Path) -> Result<()> {
    let resp = client()
        .get(url)
        .send()
        .with_context(|| format!("failed to GET {url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error downloading {url}"))?;

    let total = resp.content_length().unwrap_or(0);

    let pb = if total > 0 {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40}] {bytes}/{total_bytes} ({eta})")
                .expect("invalid template")
                .progress_chars("=> "),
        );
        pb.set_message(
            dest.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_message("Downloading...");
        pb
    };

    let mut file = File::create(dest)
        .with_context(|| format!("failed to create {}", dest.display()))?;

    let mut reader = resp;
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf).context("read error during download")?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        pb.inc(n as u64);
    }

    pb.finish_and_clear();
    Ok(())
}

/// Fetch a URL and return the body as text.
pub fn fetch_text(url: &str) -> Result<String> {
    client()
        .get(url)
        .send()
        .with_context(|| format!("failed to GET {url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error fetching {url}"))?
        .text()
        .context("failed to read response body")
}

/// Fetch a URL and deserialize the JSON response.
pub fn fetch_json<T: DeserializeOwned>(url: &str) -> Result<T> {
    client()
        .get(url)
        .send()
        .with_context(|| format!("failed to GET {url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error fetching {url}"))?
        .json::<T>()
        .context("failed to parse JSON response")
}

/// Compute SHA256 of a file and compare to expected hex hash.
pub fn verify_sha256(file_path: &Path, expected_hex: &str) -> Result<()> {
    let mut file = File::open(file_path)
        .with_context(|| format!("failed to open {}", file_path.display()))?;

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    let actual = format!("{:x}", hasher.finalize());
    let expected = expected_hex.to_lowercase();

    if actual != expected {
        bail!(
            "SHA256 mismatch for {}:\n  expected: {expected}\n  actual:   {actual}",
            file_path.display()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn verify_sha256_correct() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello world\n").unwrap();

        // sha256 of "hello world\n"
        let expected = "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447";
        verify_sha256(&path, expected).expect("should match");
    }

    #[test]
    fn verify_sha256_mismatch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello world\n").unwrap();

        let wrong = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_sha256(&path, wrong);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("SHA256 mismatch"), "error: {err}");
    }
}
