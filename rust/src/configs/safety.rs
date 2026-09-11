use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::ConfigEntry;

/// Validate the complete active manifest before name selection or mutation.
/// Targets are directory entries: resolve their parents, but never follow the
/// final symlink (an ordinary managed link intentionally points into configs/).
/// Reject parent/child targets so an earlier operation cannot redirect a later
/// one. Backups occupy the same namespace and must not alias another target.
pub(super) fn validate_layout(entries: &[ConfigEntry], project_root: &Path) -> Result<()> {
    let configs = project_root.join("configs");
    let protected = [
        resolve_entry_path(&configs)?,
        std::fs::canonicalize(&configs).context("resolving repository configs directory")?,
    ];
    let mut occupied: Vec<(TargetPath, &str)> = Vec::new();

    for entry in entries {
        let mut backup = entry.target.as_os_str().to_os_string();
        backup.push(".bak");
        for path in [&entry.target, &PathBuf::from(backup)] {
            let target = TargetPath::resolve(path)?;
            if protected
                .iter()
                .any(|root| overlaps(&target.location, root))
            {
                bail!(
                    "Config target or backup {} overlaps the repository configs directory; source files must remain separate from targets",
                    path.display()
                );
            }
            for (previous, name) in &occupied {
                if target.conflicts_with(previous) {
                    bail!(
                        "Overlapping config targets or backups for '{}' and '{}': {} and {}; targets must not alias or contain each other",
                        name,
                        entry.name,
                        previous.location.display(),
                        target.location.display()
                    );
                }
            }
            occupied.push((target, &entry.name));
        }
    }
    Ok(())
}

struct TargetPath {
    location: PathBuf,
    parents: Vec<PathBuf>,
}

impl TargetPath {
    fn resolve(path: &Path) -> Result<Self> {
        // Retain each parent directory entry before following its final
        // symlink. Otherwise a currently wrong parent symlink hides the fact
        // that replacing that entry will redirect a later target.
        let parents = path
            .ancestors()
            .skip(1)
            .map(resolve_entry_path)
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            location: resolve_entry_path(path)?,
            parents,
        })
    }

    fn conflicts_with(&self, other: &Self) -> bool {
        overlaps(&self.location, &other.location)
            || self.parents.contains(&other.location)
            || other.parents.contains(&self.location)
    }
}

fn overlaps(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}

fn resolve_entry_path(path: &Path) -> Result<PathBuf> {
    let Some(name) = path.file_name() else {
        return std::fs::canonicalize(path)
            .with_context(|| format!("resolving config path {}", path.display()));
    };
    let parent = path.parent().context("config path has no parent")?;
    Ok(resolve_directory(parent)?.join(name))
}

/// Resolve the existing portion of a directory, retaining missing components.
/// A dangling symlink or inaccessible parent is an error, not a missing path
/// that can safely be created later.
fn resolve_directory(path: &Path) -> Result<PathBuf> {
    match std::fs::canonicalize(path) {
        Ok(resolved) => Ok(resolved),
        Err(error)
            if error.kind() == ErrorKind::NotFound
                && std::fs::symlink_metadata(path)
                    .is_err_and(|error| error.kind() == ErrorKind::NotFound) =>
        {
            let parent = path
                .parent()
                .context("config path has no existing ancestor")?;
            let name = path.file_name().context("config directory has no name")?;
            Ok(resolve_directory(parent)?.join(name))
        }
        Err(error) => {
            Err(error).with_context(|| format!("resolving config parent {}", path.display()))
        }
    }
}
