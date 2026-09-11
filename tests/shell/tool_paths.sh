#!/usr/bin/env bash

set -euo pipefail
project_root=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
test_root=$(mktemp -d "${TMPDIR:-/tmp}/bashc-tool-paths.XXXXXX")
trap 'rm -rf "$test_root"' EXIT HUP INT TERM

brew_prefix="$test_root/brew prefix"
system_bin="$test_root/system-bin"
jdk_bin="$brew_prefix/opt/openjdk/bin"
mkdir -p "$brew_prefix/bin" "$brew_prefix/sbin" "$jdk_bin" "$system_bin" "$test_root/home"
cat > "$brew_prefix/bin/brew" <<'EOF'
#!/bin/sh
case "$*" in
    --prefix) printf '%s\n' "$HOMEBREW_PREFIX" ;;
    '--prefix openjdk') printf '%s/opt/openjdk\n' "$HOMEBREW_PREFIX" ;;
    *) exit 1 ;;
esac
EOF
printf '#!/bin/sh\nprintf "brew-rg\\n"\n' > "$brew_prefix/bin/rg"
printf '#!/bin/sh\nexit 1\n' > "$system_bin/java"
printf '#!/bin/sh\nprintf "working-jdk\\n"\n' > "$jdk_bin/java"
chmod +x "$brew_prefix/bin/brew" "$brew_prefix/bin/rg" "$system_bin/java" "$jdk_bin/java"

# Accept an explicit shell list for targeted local checks; CI runs both.
if [ "$#" -eq 0 ]; then
    set -- bash zsh
fi
for shell_name in "$@"; do
    shell_path=$(command -v "$shell_name")
    # The child shell must expand these variables, not this test driver.
    # shellcheck disable=SC2016
    env \
        HOME="$test_root/home" \
        NVM_DIR="$test_root/home/.nvm" \
        BASHC_INSTALL_DIR="$test_root/home/.mybin" \
        HOMEBREW_PREFIX="$brew_prefix" \
        PATH="$system_bin:/usr/bin:/bin:$jdk_bin" \
        PROJECT_ROOT="$project_root" \
        EXPECTED_JDK="$jdk_bin" \
        "$shell_path" -c '
            . "$PROJECT_ROOT/general_functions.sh" || exit 1
            determine_running_shell
            IS_MAC=false
            . "$PROJECT_ROOT/standard_settings.sh" || exit 1
            test "$(command -v brew)" = "$HOMEBREW_PREFIX/bin/brew" || exit 1
            test "$(rg)" = brew-rg || exit 1
            test "$(command -v java)" = "$EXPECTED_JDK/java" || exit 1
            test "$(java -version)" = working-jdk || exit 1
            initial_path=$PATH
            . "$PROJECT_ROOT/standard_settings.sh" || exit 1
            test "$PATH" = "$initial_path" || exit 1

            PATH="/usr/bin::$EXPECTED_JDK:$EXPECTED_JDK:"
            _bashc_prepend_path_dir "$EXPECTED_JDK"
            test "$PATH" = "$EXPECTED_JDK:/usr/bin::" || exit 1
        '
    printf 'tool PATH checks passed: %s\n' "$shell_name"
done
