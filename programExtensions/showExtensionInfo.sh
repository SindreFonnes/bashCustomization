#!/bin/bash
# Generic show script for program extensions.
# Usage: showExtensionInfo.sh <extension_name> <extension_dir>
# Parses alias and function definitions from all .sh files in the extension directory.

extension_name="$1"
extension_dir="$2"

if [[ -z "$extension_name" || -z "$extension_dir" ]]; then
    echo "Usage: showExtensionInfo.sh <name> <dir>"
    exit 1
fi

echo "$extension_name extensions:"
echo ""

# Find aliases
aliases=()
while IFS= read -r line; do
    # Strip leading whitespace and "alias " prefix, remove trailing semicolons
    cleaned="${line#*alias }"
    cleaned="${cleaned%;}"
    aliases+=("$cleaned")
done < <(grep -rh "^[[:space:]]*alias " "$extension_dir" --include="*.sh" 2>/dev/null | grep -v "^[[:space:]]*#")

if [[ ${#aliases[@]} -gt 0 ]]; then
    echo "  Aliases:"
    for a in "${aliases[@]}"; do
        echo "    $a"
    done
    echo ""
fi

# Find functions
functions=()
while IFS= read -r line; do
    # Extract function name from "name () {" or "name() {" patterns
    func_name="${line%%(*}"
    func_name="${func_name## }"
    func_name="${func_name%% }"
    functions+=("$func_name")
done < <(grep -rh "^[a-zA-Z_-][a-zA-Z0-9_-]* *() *{" "$extension_dir" --include="*.sh" 2>/dev/null | grep -v "^[[:space:]]*#" | grep -v "Show.sh")

if [[ ${#functions[@]} -gt 0 ]]; then
    echo "  Functions:"
    for f in "${functions[@]}"; do
        echo "    $f"
    done
    echo ""
fi

if [[ ${#aliases[@]} -eq 0 && ${#functions[@]} -eq 0 ]]; then
    echo "  No aliases or functions defined."
fi
