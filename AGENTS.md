- This is a shell customization framework (bash/zsh) that provides aliases, functions, install scripts, and program extensions
- The project is sourced from `main.sh` and loads modules via `load_shell_extentionfiles` in `general_functions.sh`
- DO NOT: break the sourcing chain — every script must be sourceable without errors, as failures cascade
- DO NOT: use bashisms in files that need to work with both bash and zsh (check `PROFILE_SHELL` or use POSIX-compatible syntax)
- USE: `shellcheck` to validate shell scripts before committing
- Be aware of the cross-platform nature: scripts must handle macOS (`IS_MAC`), WSL (`IS_WSL`), and native Linux

## Project structure
- `main.sh` — entry point, sourced by `.bashrc`/`.zshrc`
- `general_functions.sh` — core utilities (OS detection, shell detection, module loader)
- `variables.sh` — shared path variables and standard program settings
- `standard_settings.sh` — shell config, plugins, PATH exports
- `shellFunctionality/` — shell aliases and functions
- `programExtensions/` — per-program extensions (git, rust, bun, nvim, etc.)
- `installScripts/` — install helpers for tools and languages
- `generalScripts/` — standalone utility scripts (git config, SSL certs, GPG, etc.)
- `local/` — machine-specific overrides (not committed)

## Conventions
- Follow existing naming patterns: `snake_case` for functions, lowercase with underscores for variables
- New program extensions go in `programExtensions/<program>/` with their own sourcing structure
- New install scripts go in `installScripts/<tool>/`
- Keep scripts modular — one concern per file
- Test changes by running `source main.sh` in a new shell session to verify nothing breaks

## Future Rust migration context
- Parts of this project may be rewritten in Rust in the future (e.g., performance-critical functions, cross-platform logic, install orchestration)
- When making changes, prefer clean interfaces between modules — functions with clear inputs/outputs are easier to port
- Avoid deeply nested shell logic that would be hard to translate; prefer straightforward control flow
- Document any non-obvious shell behavior that a Rust rewrite would need to replicate
- Keep platform detection logic centralized (in `general_functions.sh`) so it can be replaced by a single Rust binary later

## When making changes
- Before committing, run `shellcheck` on modified `.sh` files and fix any warnings
- After editing, verify the sourcing chain works: `source main.sh` should complete without errors
- If you make a change that would affect how other modules load, trace the full loading path in `load_shell_extentionfiles`

## Writing specs and plans
- Specs and plans should be **requirements-driven, not implementation-prescriptive**. Describe *what* the system should do, *why*, and what constraints apply — not *how* to code it. Avoid including full code implementations in specs; the implementing agent should make code-level decisions based on the requirements and the current state of the codebase. Code snippets are acceptable only for interface contracts (e.g., a trait signature) or to illustrate a concept that would be ambiguous in prose. Detailed code in a spec biases the implementer toward a specific solution that may not be correct once they see the actual codebase, and it creates a false sense of completeness when the real details only emerge during implementation.

If you notice something does not quite make sense, or you loop back to the same issue multiple times, consider raising the issue to a human, and add documentation on the solution that is settled on.
If you notice you have to look for something multiple times or feel you have an insight regarding the work done, add it to a `potential_insights.md` file at the project root so that they can be looked through and potentially converted into documentation later by a human.
