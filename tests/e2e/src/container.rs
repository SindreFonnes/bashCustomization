use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::BuildImageOptions;
use bollard::Docker;
use futures_util::StreamExt;
use std::io::Read as _;
use std::path::Path;

/// Result of executing a command inside a container.
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
}

/// Manages the lifecycle of a single Docker container for E2E testing.
pub struct TestContainer {
    docker: Docker,
    container_id: String,
}

impl TestContainer {
    /// Build a Docker image from a Dockerfile and build context directory.
    ///
    /// Images are rebuilt by default so source changes cannot accidentally be
    /// tested against a stale binary. Set `REUSE_E2E_IMAGES=1` to opt into
    /// local cache reuse; `REBUILD_IMAGES=1` overrides that opt-in.
    pub async fn build_image(
        docker: &Docker,
        image_tag: &str,
        dockerfile: &str,
        build_context_path: &Path,
    ) -> Result<()> {
        let reuse_requested =
            std::env::var("REUSE_E2E_IMAGES").is_ok() && std::env::var("REBUILD_IMAGES").is_err();
        if reuse_requested && docker.inspect_image(image_tag).await.is_ok() {
            return Ok(());
        }

        let tar_bytes = create_tar_archive(build_context_path)
            .with_context(|| format!("creating tar archive of {}", build_context_path.display()))?;

        let options = BuildImageOptions {
            t: image_tag.to_string(),
            dockerfile: dockerfile.to_string(),
            rm: true,
            ..Default::default()
        };

        let mut stream = docker.build_image(options, None, Some(tar_bytes.into()));

        while let Some(msg) = stream.next().await {
            let info = msg.context("docker image build stream error")?;
            if let Some(err) = info.error {
                anyhow::bail!("Docker build error: {err}");
            }
        }

        Ok(())
    }

    /// Extract the `/bashc` binary from the builder image.
    ///
    /// Creates a temporary container from `builder_image_tag`, copies the file out,
    /// and writes it to `dest_path`.
    pub async fn extract_binary(
        docker: &Docker,
        builder_image_tag: &str,
        dest_path: &Path,
    ) -> Result<()> {
        let container_name = format!("bashc-binary-extractor-{}", std::process::id());

        // Create a container (do not start it -- we just need the filesystem).
        let config = Config {
            image: Some(builder_image_tag.to_string()),
            cmd: Some(vec!["/bin/true".to_string()]),
            ..Default::default()
        };
        let options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        // Remove any leftover extractor container from a previous run.
        let _ = docker
            .remove_container(
                &container_name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        docker
            .create_container(Some(options), config)
            .await
            .context("create extractor container")?;

        // Ensure cleanup on all exit paths — if any step below fails,
        // we still remove the container (best-effort).
        let result = Self::extract_binary_inner(docker, &container_name, dest_path).await;

        // Clean up the extractor container regardless of success/failure.
        let _ = docker
            .remove_container(
                &container_name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        result
    }

    /// Inner extraction logic — separated so cleanup runs on all exit paths.
    async fn extract_binary_inner(
        docker: &Docker,
        container_name: &str,
        dest_path: &Path,
    ) -> Result<()> {
        let tar_stream = docker.download_from_container(
            container_name,
            Some(bollard::container::DownloadFromContainerOptions { path: "/bashc" }),
        );

        let mut tar_bytes: Vec<u8> = Vec::new();
        let mut stream = tar_stream;
        while let Some(chunk) = stream.next().await {
            let data = chunk.context("reading tar stream from container")?;
            tar_bytes.extend_from_slice(&data);
        }

        let mut archive = tar::Archive::new(tar_bytes.as_slice());
        let mut entries = archive.entries().context("reading tar entries")?;
        let mut entry = entries
            .next()
            .context("no file found in container at /bashc — builder image may be broken")?
            .context("reading tar entry")?;
        let mut contents = Vec::new();
        entry
            .read_to_end(&mut contents)
            .context("reading binary from tar entry")?;
        std::fs::write(dest_path, &contents)
            .with_context(|| format!("writing binary to {}", dest_path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(dest_path, perms)?;
        }

        Ok(())
    }

    /// Create and start a container from the given image.
    ///
    /// The container runs `tail -f /dev/null` to stay alive.
    pub async fn create_and_start(
        docker: &Docker,
        image_tag: &str,
        container_name: &str,
    ) -> Result<Self> {
        // Remove any leftover container with the same name.
        let _ = docker
            .remove_container(
                container_name,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        let config = Config {
            image: Some(image_tag.to_string()),
            cmd: Some(vec![
                "tail".to_string(),
                "-f".to_string(),
                "/dev/null".to_string(),
            ]),
            ..Default::default()
        };
        let options = CreateContainerOptions {
            name: container_name.to_string(),
            ..Default::default()
        };

        let response = docker
            .create_container(Some(options), config)
            .await
            .context("creating test container")?;

        docker
            .start_container(&response.id, None::<StartContainerOptions<String>>)
            .await
            .context("starting test container")?;

        Ok(Self {
            docker: docker.clone(),
            container_id: response.id,
        })
    }

    /// Execute a command inside the running container and return structured results.
    pub async fn exec(&self, cmd: &[&str]) -> Result<ExecResult> {
        let exec_config = CreateExecOptions {
            cmd: Some(cmd.iter().map(|s| s.to_string()).collect()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self
            .docker
            .create_exec(&self.container_id, exec_config)
            .await
            .context("creating exec")?;

        let start_result = self
            .docker
            .start_exec(&exec.id, None)
            .await
            .context("starting exec")?;

        let mut stdout = String::new();
        let mut stderr = String::new();

        if let StartExecResults::Attached { mut output, .. } = start_result {
            while let Some(msg) = output.next().await {
                let chunk = msg.context("reading exec output")?;
                match chunk {
                    bollard::container::LogOutput::StdOut { message } => {
                        stdout.push_str(&String::from_utf8_lossy(&message));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        stderr.push_str(&String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
        }

        // Inspect the exec to get the exit code.
        let inspect = self
            .docker
            .inspect_exec(&exec.id)
            .await
            .context("inspecting exec result")?;

        let exit_code = inspect.exit_code.unwrap_or(-1);

        Ok(ExecResult {
            stdout,
            stderr,
            exit_code,
        })
    }

    /// Stop and remove the container.
    pub async fn cleanup(self) -> Result<()> {
        let _ = self
            .docker
            .stop_container(&self.container_id, Some(StopContainerOptions { t: 5 }))
            .await;

        self.docker
            .remove_container(
                &self.container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await
            .context("removing test container")?;

        Ok(())
    }
}

/// Directories to exclude from the Docker build context tar archive.
const TAR_EXCLUDE_DIRS: &[&str] = &[
    ".git",
    "target",
    "rust/target",
    "tests/e2e/target",
    "node_modules",
];

/// Create a tar archive from a directory, suitable for sending to the Docker build API.
///
/// Excludes `.git`, `target`, and other large directories that are not needed
/// for Docker builds.
fn create_tar_archive(context_path: &Path) -> Result<Vec<u8>> {
    let mut archive = tar::Builder::new(Vec::new());
    append_dir_filtered(&mut archive, context_path, context_path)?;
    let bytes = archive.into_inner().context("finalizing tar archive")?;
    Ok(bytes)
}

/// Recursively append directory entries to a tar archive, skipping excluded dirs.
fn append_dir_filtered(
    archive: &mut tar::Builder<Vec<u8>>,
    base: &Path,
    current: &Path,
) -> Result<()> {
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("reading directory {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(base).with_context(|| {
            format!(
                "archive entry {} escaped build context {}",
                path.display(),
                base.display()
            )
        })?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("reading file type for {}", path.display()))?;

        // Skip excluded directories.
        if file_type.is_dir()
            && TAR_EXCLUDE_DIRS
                .iter()
                .any(|excluded| rel == Path::new(excluded))
        {
            continue;
        }

        // Use directory-entry metadata rather than Path::is_dir(), which
        // follows symlinks. A repository symlink must be archived as a link,
        // never traversed outside the checked-out build context.
        if file_type.is_dir() {
            append_dir_filtered(archive, base, &path)?;
        } else {
            archive
                .append_path_with_name(&path, rel)
                .with_context(|| format!("adding {} to tar", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn build_context_does_not_follow_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let context = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "outside").unwrap();
        symlink(outside.path(), context.path().join("external-link")).unwrap();

        let bytes = create_tar_archive(context.path()).unwrap();
        let mut archive = tar::Archive::new(bytes.as_slice());
        let paths = archive
            .entries()
            .unwrap()
            .map(|entry| entry.unwrap().path().unwrap().into_owned())
            .collect::<Vec<_>>();

        assert!(paths.iter().any(|path| path == Path::new("external-link")));
        assert!(!paths
            .iter()
            .any(|path| path.starts_with("external-link/") && path != Path::new("external-link")));
    }

    #[test]
    fn build_context_excludes_e2e_compilation_output() {
        let context = tempfile::tempdir().unwrap();
        let target = context.path().join("tests/e2e/target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("artifact"), "large build output").unwrap();
        std::fs::write(context.path().join("kept"), "source").unwrap();

        let bytes = create_tar_archive(context.path()).unwrap();
        let mut archive = tar::Archive::new(bytes.as_slice());
        let paths = archive
            .entries()
            .unwrap()
            .map(|entry| entry.unwrap().path().unwrap().into_owned())
            .collect::<Vec<_>>();

        assert!(paths.iter().any(|path| path == Path::new("kept")));
        assert!(!paths
            .iter()
            .any(|path| path.starts_with("tests/e2e/target")));
    }
}
