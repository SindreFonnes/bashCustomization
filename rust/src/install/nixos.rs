use anyhow::{Result, bail};

/// CLI selectors, package attributes, and NixOS module options are distinct
/// interfaces. Keep the mapping explicit, including multi-package toolchains.
pub(super) fn guidance(tool: &str) -> Result<String> {
    let packages = match tool {
        "base" => "gcc gnumake pkg-config git gnupg curl zip unzip",
        "go" => "go",
        "rust" => "rustc cargo",
        "docker" => "docker",
        "azure" => "azure-cli",
        "dotnet" => "dotnet-sdk",
        "neovim" => "neovim",
        "obsidian" => "obsidian",
        "java" => "jdk",
        "github" => "gh",
        "terraform" => "terraform",
        "postgres" => "postgresql",
        "kubectl" => "kubectl",
        "ripgrep" => "ripgrep",
        "bat" => "bat",
        "fd" => "fd",
        "eza" => "eza",
        "shellcheck" => "shellcheck",
        "nerd-font" => "nerd-fonts.jetbrains-mono",
        "javascript" => "nodejs pnpm bun yarn",
        _ => bail!("No NixOS configuration mapping for installer '{tool}'"),
    };
    let option = if tool == "nerd-font" {
        "fonts.packages"
    } else {
        "environment.systemPackages"
    };
    let mut configuration = format!("{option} = with pkgs; [ {packages} ];");
    if tool == "docker" {
        configuration.push_str("\nvirtualisation.docker.enable = true;");
    }
    let note = match tool {
        "obsidian" | "terraform" => {
            "\nThis package requires allowing its unfree license in your Nixpkgs configuration."
        }
        "javascript" => "\nNode.js is selected through Nixpkgs; this route does not install nvm.",
        "rust" => {
            "\nThe Rust toolchain is selected through Nixpkgs; this route does not install rustup."
        }
        "postgres" => {
            "\nFor a local database server, also configure `services.postgresql.enable = true;`."
        }
        _ => "",
    };
    Ok(format!(
        "NixOS: Merge into your configuration.nix:\n{configuration}{note}\nThen run `nixos-rebuild switch`."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::platform::{Arch, Distro, Os, Platform};
    use crate::install::{ALL_TOOLS, Installer};

    #[test]
    fn every_applicable_installer_has_explicit_guidance() {
        let platform = Platform {
            os: Os::Linux(Distro::NixOs),
            arch: Arch::X86_64,
        };
        for tool in ALL_TOOLS
            .iter()
            .filter(|tool| tool.is_applicable(&platform))
        {
            assert!(
                guidance(tool.name()).is_ok(),
                "{} has no NixOS mapping",
                tool.name()
            );
        }
        assert!(guidance("unknown-tool").is_err());
    }

    #[test]
    fn selectors_map_to_packages_and_services() {
        for (tool, expected) in [
            ("github", "[ gh ]"),
            ("azure", "[ azure-cli ]"),
            ("java", "[ jdk ]"),
            ("rust", "[ rustc cargo ]"),
            ("postgres", "[ postgresql ]"),
            ("javascript", "[ nodejs pnpm bun yarn ]"),
            (
                "nerd-font",
                "fonts.packages = with pkgs; [ nerd-fonts.jetbrains-mono ];",
            ),
            ("docker", "virtualisation.docker.enable = true;"),
        ] {
            assert!(
                guidance(tool).unwrap().contains(expected),
                "incorrect mapping for {tool}"
            );
        }
    }
}
