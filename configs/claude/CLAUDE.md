# Developer Guidelines
## Use branches and pull requests
When being asked to develop a feature, ask if you should use a branch or not.
After the user is happy with the feature, make a pull request and wait for approval.

# Tool & Performance Guidelines

### Preferred CLI Tools
Whenever performing shell operations, prefer these modern alternatives for speed and better output:
- **Search:** Use `rg` (ripgrep) instead of `grep`. Always use `--smart-case`.
- **Find:** Use `fd` instead of `find` for file searching.
- **List:** Use `eza` or `exa` instead of `ls` if available.
- **View:** Use `bat` instead of `cat` for syntax highlighting when reading files.