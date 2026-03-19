# Known Issues

## Ubuntu-specific assumptions in installers

Several installers assume Ubuntu when they should support both Ubuntu and
plain Debian (and eventually other Debian derivatives). These work correctly
on Ubuntu but may fail or produce incorrect results on Debian.

### Docker (`src/install/tools/docker.rs`)

**Severity: Bug**

The apt repo URL is hardcoded to `download.docker.com/linux/ubuntu`. On plain
Debian it should use `download.docker.com/linux/debian`. The GPG key URL is
also Ubuntu-specific. The codename falls back to `"jammy"` (Ubuntu 22.04) when
`VERSION_CODENAME` is missing — on Debian this should fall back to `"bookworm"`
or detect based on `ID`.

**Fix:** Read `ID` from `/etc/os-release` (already parsed by `Platform`) and
use the correct Docker repo path (`linux/ubuntu` vs `linux/debian`). Each has
different codename schemes.

### Azure CLI (`src/install/tools/azure.rs`)

**Severity: Minor**

Falls back to codename `"jammy"` when `VERSION_CODENAME` is not found. The
Azure CLI apt repo is distro-agnostic (works with codenames from both Ubuntu
and Debian), but if the codename detection fails it would use an Ubuntu
codename on Debian. In practice `VERSION_CODENAME` is almost always present
in `/etc/os-release`, so this rarely triggers.

### .NET SDK (`src/install/tools/dotnet.rs`)

**Severity: Minor**

Same fallback-to-`"jammy"` issue. Microsoft's dotnet repo may not have
packages for all Debian codenames, and using an Ubuntu codename on Debian
could install incompatible packages.

### Terraform (`src/install/tools/terraform.rs`)

**Severity: Minor**

Falls back to codename `"jammy"`. HashiCorp's repo supports both Ubuntu and
Debian codenames, but using the wrong one would fail to find packages.

### Base packages (`src/install/tools/base.rs`)

**Severity: Minor**

- Calls `add-apt-repository universe -y` — the `universe` repo is
  Ubuntu-specific. Debian does not have a `universe` repo (equivalent packages
  are in `main`). The `add-apt-repository` command may not exist on plain
  Debian without `software-properties-common`. Currently wrapped in
  `let _ =` so failures are silently ignored.
- Includes `nala` in the package list — may not be available in all Debian
  versions or repos.
- Includes `safe-rm` — may not be in all Debian repos.

## Recommended fix

The `Platform` struct already knows the distro and could differentiate Ubuntu
from Debian. A shared helper like `get_apt_codename(platform)` could return
the correct codename and repo base URL based on both `ID` and
`VERSION_CODENAME` from `/etc/os-release`, eliminating the `"jammy"` fallback
across all four installers.

For base.rs, the `universe` repo command should be guarded behind an Ubuntu
check (not just Debian-family), and the package list should be reviewed for
Debian compatibility.
