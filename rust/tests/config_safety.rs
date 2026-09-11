use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Fixture {
    _root: tempfile::TempDir,
    home: PathBuf,
    repo: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("home");
        let repo = root.path().join("repo");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(repo.join("configs/tree")).unwrap();
        fs::write(repo.join("configs/example"), "original source\n").unwrap();
        Self {
            _root: root,
            home,
            repo,
        }
    }

    fn manifest(&self, entries: &[(&str, &str)]) {
        let content: String = entries
            .iter()
            .enumerate()
            .map(|(i, (source, target))| {
                format!(
                    "[[config]]\nname = \"entry-{i}\"\nsource = {source:?}\ntarget = {target:?}\n\n"
                )
            })
            .collect();
        fs::write(self.repo.join("configs/manifest.toml"), content).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_bashc"))
            .arg("configs")
            .args(args)
            .env("HOME", &self.home)
            .env("BASHC_ROOT", &self.repo)
            .output()
            .unwrap()
    }

    fn rejected(&self, args: &[&str]) {
        let output = self.run(args);
        assert!(
            !output.status.success(),
            "unexpected success: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .to_lowercase()
                .contains("overlap"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            fs::read_to_string(self.repo.join("configs/example")).unwrap(),
            "original source\n"
        );
    }
}

#[test]
fn nested_targets_are_rejected_before_any_mutation_in_either_order() {
    let fixture = Fixture::new();
    for entries in [
        [
            ("tree", "~/.config/tool"),
            ("example", "~/.config/tool/child"),
        ],
        [
            ("example", "~/.config/tool/child"),
            ("tree", "~/.config/tool"),
        ],
    ] {
        fixture.manifest(&entries);
        for args in [
            vec!["check"],
            vec!["link", "--force", "discard"],
            vec!["unlink", "--yes"],
            vec!["link", "entry-0"],
        ] {
            fixture.rejected(&args);
            assert!(!fixture.home.join(".config").exists());
            assert!(!fixture.repo.join("configs/tree/child").exists());
        }
    }
}

#[test]
fn sources_and_their_ancestors_cannot_be_targets_even_with_external_authority() {
    let fixture = Fixture::new();
    for target in [
        fixture.repo.join("configs/example"),
        fixture.repo.join("configs"),
        fixture.repo.clone(),
    ] {
        fixture.manifest(&[("example", target.to_str().unwrap())]);
        for strategy in ["replace", "discard", "keep"] {
            fixture.rejected(&["link", "--force", strategy, "--allow-outside-home"]);
        }
        fixture.rejected(&["check"]);
        fixture.rejected(&["unlink", "--yes", "--allow-outside-home"]);
    }
}

#[test]
fn parent_symlink_alias_into_sources_is_rejected() {
    let fixture = Fixture::new();
    symlink(fixture.repo.join("configs"), fixture.home.join("alias")).unwrap();
    fixture.manifest(&[("example", "~/alias/example")]);
    fixture.rejected(&["link", "--force", "discard", "--allow-outside-home"]);
}

#[test]
fn aliased_targets_and_backup_collisions_are_rejected() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.home.join("real")).unwrap();
    symlink(fixture.home.join("real"), fixture.home.join("alias")).unwrap();
    for target in [
        "~/alias/config",
        "~/alias/config.bak",
        "~/alias/config/child",
    ] {
        fixture.manifest(&[("example", "~/real/config"), ("example", target)]);
        fixture.rejected(&["link", "--force", "replace"]);
        assert!(!fixture.home.join("real/config").exists());
    }
}

#[test]
fn ordinary_managed_links_and_backup_restore_still_work() {
    let fixture = Fixture::new();
    fixture.manifest(&[("example", "~/config")]);
    fs::write(fixture.home.join("config"), "local override\n").unwrap();
    for args in [
        vec!["link", "--force", "replace"],
        vec!["check"],
        vec!["status"],
        vec!["unlink", "--yes"],
    ] {
        let output = fixture.run(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert_eq!(
        fs::read_to_string(fixture.home.join("config")).unwrap(),
        "local override\n"
    );
    assert!(!fixture.home.join("config.bak").exists());
}

#[test]
fn replacing_a_parent_symlink_cannot_redirect_another_target() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.home.join("real")).unwrap();
    fs::create_dir(fixture.home.join("unmanaged")).unwrap();
    symlink(fixture.home.join("real"), fixture.home.join("alias")).unwrap();
    symlink(
        fixture.home.join("unmanaged"),
        fixture.home.join("real/tool"),
    )
    .unwrap();
    fixture.manifest(&[("tree", "~/real/tool"), ("example", "~/alias/tool/child")]);
    fixture.rejected(&["link", "--force", "discard"]);
    assert!(!fixture.repo.join("configs/tree/child").exists());
    assert_eq!(
        fs::read_link(fixture.home.join("real/tool")).unwrap(),
        fixture.home.join("unmanaged")
    );
}

#[test]
fn dangling_links_can_be_unlinked_when_source_is_missing() {
    let fixture = Fixture::new();
    fixture.manifest(&[("missing", "~/config")]);
    symlink(
        fixture.repo.join("configs/missing"),
        fixture.home.join("config"),
    )
    .unwrap();
    let output = fixture.run(&["unlink", "--yes"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fs::symlink_metadata(fixture.home.join("config")).is_err());
}

#[test]
fn disjoint_platform_targets_do_not_conflict() {
    let fixture = Fixture::new();
    fs::write(
        fixture.repo.join("configs/manifest.toml"),
        "\
[[config]]
name = 'test'
source = 'example'
target = '~/config'
platform = 'linux'
[[config]]
name = 'test'
source = 'example'
target = '~/config'
platform = 'macos'
",
    )
    .unwrap();
    let output = fixture.run(&["check"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture.home.join("config").is_symlink());
}
