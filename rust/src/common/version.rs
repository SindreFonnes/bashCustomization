use anyhow::{Context, Result};
use semver::Version;

/// Parse a version string, stripping common prefixes (v, go).
pub fn parse(version_str: &str) -> Result<Version> {
    let stripped = version_str
        .strip_prefix("go")
        .or_else(|| version_str.strip_prefix('v'))
        .unwrap_or(version_str);

    Version::parse(stripped)
        .with_context(|| format!("failed to parse version: {version_str:?} (stripped: {stripped:?})"))
}

/// Returns true if `new_ver` is greater than `current`.
pub fn is_newer(current: &str, new_ver: &str) -> Result<bool> {
    let current = parse(current)?;
    let new = parse(new_ver)?;
    Ok(new > current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_v_prefix() {
        let v = parse("v1.22.0").unwrap();
        assert_eq!(v, Version::new(1, 22, 0));
    }

    #[test]
    fn strips_go_prefix() {
        let v = parse("go1.22.0").unwrap();
        assert_eq!(v, Version::new(1, 22, 0));
    }

    #[test]
    fn parses_plain_version() {
        let v = parse("3.14.1").unwrap();
        assert_eq!(v, Version::new(3, 14, 1));
    }

    #[test]
    fn is_newer_works() {
        assert!(is_newer("1.21.0", "1.22.0").unwrap());
        assert!(!is_newer("1.22.0", "1.21.0").unwrap());
    }

    #[test]
    fn equal_versions_not_newer() {
        assert!(!is_newer("1.22.0", "1.22.0").unwrap());
    }

    #[test]
    fn is_newer_with_prefixes() {
        assert!(is_newer("go1.21.0", "go1.22.0").unwrap());
        assert!(is_newer("v1.0.0", "v2.0.0").unwrap());
    }
}
