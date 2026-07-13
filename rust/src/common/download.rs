use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use super::command;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_ATTEMPTS: usize = 3;

fn client() -> Result<Client> {
    Client::builder()
        .user_agent("bashc/0.1.0")
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .context("failed to build HTTP client")
}

fn response(client: &Client, url: &str) -> reqwest::Result<Response> {
    client.get(url).send()?.error_for_status()
}

fn should_retry(error: &reqwest::Error) -> bool {
    match error.status() {
        Some(status) => is_retryable_status(status),
        None => true,
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status.is_server_error()
        || status == StatusCode::REQUEST_TIMEOUT
        || status == StatusCode::TOO_MANY_REQUESTS
}

fn retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(200 * attempt as u64)
}

fn report_retry(url: &str, attempt: usize, error: &dyn std::fmt::Display) {
    let delay = retry_delay(attempt);
    eprintln!(
        "Download attempt {attempt}/{MAX_ATTEMPTS} failed for {url}: {error}. Retrying in {}ms...",
        delay.as_millis()
    );
    thread::sleep(delay);
}

fn progress_bar(total: u64, destination: &Path) -> ProgressBar {
    if total > 0 {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40}] {bytes}/{total_bytes} ({eta})")
                .expect("invalid template")
                .progress_chars("=> "),
        );
        pb.set_message(
            destination
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
        );
        pb
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_message("Downloading...");
        pb
    }
}

/// Download a URL to a same-directory staging file with a progress bar.
///
/// The destination is replaced atomically only after a complete response has
/// been received and flushed, so a failed download cannot truncate a working
/// file that was already present.
pub fn download_file(url: &str, dest: &Path) -> Result<()> {
    let client = client()?;
    let parent = dest
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    'attempts: for attempt in 1..=MAX_ATTEMPTS {
        let mut reader = match response(&client, url) {
            Ok(response) => response,
            Err(error) if attempt < MAX_ATTEMPTS && should_retry(&error) => {
                report_retry(url, attempt, &error);
                continue;
            }
            Err(error) => {
                return Err(error).with_context(|| format!("failed to download {url}"));
            }
        };

        let pb = progress_bar(reader.content_length().unwrap_or(0), dest);
        let mut staged = tempfile::NamedTempFile::new_in(parent).with_context(|| {
            format!(
                "failed to create download staging file in {}",
                parent.display()
            )
        })?;
        let mut buffer = [0u8; 8192];

        loop {
            let count = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => count,
                Err(error) if attempt < MAX_ATTEMPTS => {
                    pb.finish_and_clear();
                    report_retry(url, attempt, &error);
                    continue 'attempts;
                }
                Err(error) => {
                    pb.finish_and_clear();
                    return Err(error).context("read error during download");
                }
            };

            staged.write_all(&buffer[..count]).with_context(|| {
                format!(
                    "failed to write download staging file for {}",
                    dest.display()
                )
            })?;
            pb.inc(count as u64);
        }

        staged.flush().context("flushing downloaded file")?;
        staged
            .as_file()
            .sync_all()
            .context("syncing downloaded file")?;
        staged
            .persist(dest)
            .map_err(|error| error.error)
            .with_context(|| {
                format!(
                    "failed to replace {} with completed download",
                    dest.display()
                )
            })?;
        pb.finish_and_clear();
        return Ok(());
    }

    unreachable!("download retry loop always returns on its final attempt")
}

fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
    let client = client()?;

    for attempt in 1..=MAX_ATTEMPTS {
        let result = response(&client, url).and_then(|response| response.bytes());
        match result {
            Ok(body) => return Ok(body.to_vec()),
            Err(error) if attempt < MAX_ATTEMPTS && should_retry(&error) => {
                report_retry(url, attempt, &error);
            }
            Err(error) => return Err(error).with_context(|| format!("failed to fetch {url}")),
        }
    }

    unreachable!("fetch retry loop always returns on its final attempt")
}

/// Fetch a URL and return the body as text.
pub fn fetch_text(url: &str) -> Result<String> {
    let bytes = fetch_bytes(url)?;
    String::from_utf8(bytes).with_context(|| format!("response body from {url} was not UTF-8"))
}

/// Fetch a URL and deserialize the JSON response.
pub fn fetch_json<T: DeserializeOwned>(url: &str) -> Result<T> {
    let bytes = fetch_bytes(url)?;
    serde_json::from_slice(&bytes).context("failed to parse JSON response")
}

/// Compute SHA256 of a file and compare to expected hex hash.
pub fn verify_sha256(file_path: &Path, expected_hex: &str) -> Result<()> {
    let mut file =
        File::open(file_path).with_context(|| format!("failed to open {}", file_path.display()))?;

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

/// Verify a GitHub Releases asset using the digest supplied by the release API.
/// Assets without a SHA-256 digest are rejected rather than silently downgraded
/// to transport-only verification.
pub fn verify_github_asset_digest(file_path: &Path, digest: Option<&str>) -> Result<()> {
    let digest = digest.context("GitHub release metadata did not provide an asset digest")?;
    let expected = digest
        .strip_prefix("sha256:")
        .with_context(|| format!("unsupported GitHub asset digest format: {digest}"))?;
    verify_sha256(file_path, expected)
}

/// Select one exact artifact checksum from a conventional SHA-256 manifest.
pub fn sha256_from_manifest(manifest: &str, artifact_name: &str) -> Result<String> {
    let matches = manifest
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let hash = fields.next()?;
            let name = fields.next()?.trim_start_matches('*');
            (name == artifact_name && fields.next().is_none()).then_some(hash)
        })
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [hash]
            if hash.len() == 64 && hash.chars().all(|character| character.is_ascii_hexdigit()) =>
        {
            Ok((*hash).to_string())
        }
        [] => bail!("checksum manifest did not contain {artifact_name}"),
        [_] => bail!("checksum manifest contained an invalid SHA-256 value for {artifact_name}"),
        _ => bail!("checksum manifest contained duplicate entries for {artifact_name}"),
    }
}

/// Download a shell installer completely, verify its pinned digest, and only
/// then execute it. Arguments before and after the script path are separated so
/// wrappers such as `env NONINTERACTIVE=1 /bin/bash SCRIPT` remain argv-safe.
pub fn run_verified_script(
    url: &str,
    expected_sha256: &str,
    program: &str,
    before_script: &[&str],
    after_script: &[&str],
) -> Result<()> {
    let directory =
        tempfile::tempdir().context("creating temporary directory for installer script")?;
    let script_path = directory.path().join("installer.sh");
    download_file(url, &script_path)?;
    verify_sha256(&script_path, expected_sha256)?;

    let script = script_path
        .to_str()
        .with_context(|| format!("script path is not valid UTF-8: {}", script_path.display()))?;
    let mut arguments = Vec::with_capacity(before_script.len() + after_script.len() + 1);
    arguments.extend_from_slice(before_script);
    arguments.push(script);
    arguments.extend_from_slice(after_script);
    command::run_visible(program, &arguments)
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

    #[test]
    fn retry_policy_covers_transient_http_statuses() {
        assert!(is_retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(is_retryable_status(StatusCode::REQUEST_TIMEOUT));
        assert!(is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(!is_retryable_status(StatusCode::NOT_FOUND));
        assert!(!is_retryable_status(StatusCode::UNAUTHORIZED));
    }

    #[test]
    fn abandoned_staging_file_preserves_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("artifact");
        std::fs::write(&destination, b"working version").unwrap();
        let mut staged = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
        staged.write_all(b"partial replacement").unwrap();

        drop(staged);
        assert_eq!(std::fs::read(&destination).unwrap(), b"working version");
    }

    #[test]
    fn github_digest_requires_supported_sha256_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("asset");
        std::fs::write(&path, b"hello world\n").unwrap();
        let hash = "a948904f2f0f479b8f8197694b30184b0d2ed1c1cd2a1ec0fb85d299a192a447";

        verify_github_asset_digest(&path, Some(&format!("sha256:{hash}"))).unwrap();
        assert!(verify_github_asset_digest(&path, None).is_err());
        assert!(verify_github_asset_digest(&path, Some(&format!("sha512:{hash}"))).is_err());
    }

    #[test]
    fn checksum_manifest_requires_one_exact_artifact_match() {
        let hash = "ef552a3e638f25125c6ad4c51176a6adcdce295ab1d2ffacf0db060caf8c1582";
        let manifest = format!("{hash}  JetBrainsMono.tar.xz\n{hash} *Other.tar.xz\n");
        assert_eq!(
            sha256_from_manifest(&manifest, "JetBrainsMono.tar.xz").unwrap(),
            hash
        );
        assert!(sha256_from_manifest(&manifest, "JetBrainsMono.zip").is_err());
        assert!(
            sha256_from_manifest(
                &format!("{manifest}{hash}  JetBrainsMono.tar.xz\n"),
                "JetBrainsMono.tar.xz"
            )
            .is_err()
        );
    }
}
