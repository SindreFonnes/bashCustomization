use anyhow::{Result, bail};

use super::command;

/// Escalation method detected at runtime.
enum Escalator {
    Root,
    Sudo,
    Doas,
    Su,
}

/// Detect which privilege escalation method is available.
///
/// Preference order: root (no escalation needed) > sudo > doas > su.
/// Returns an error if none are found, directing the user to install doas.
fn detect_escalator() -> Result<Escalator> {
    if command::is_root() {
        return Ok(Escalator::Root);
    }

    if command::exists("sudo") {
        return Ok(Escalator::Sudo);
    }

    if command::exists("doas") {
        return Ok(Escalator::Doas);
    }

    if command::exists("su") {
        return Ok(Escalator::Su);
    }

    bail!(
        "No privilege escalation tool found (sudo, doas, or su). \
        Install doas with: bashc install doas"
    )
}

/// Run a command with privilege escalation, using whatever tool is available.
///
/// If already running as root, the command is run directly.
/// Otherwise detects sudo, doas, or su (in that order) and uses it.
///
/// Note: `su -c` requires the full command as a single string argument,
/// unlike sudo/doas which accept program + args directly.
pub fn run_privileged(program: &str, args: &[&str]) -> Result<()> {
    match detect_escalator()? {
        Escalator::Root => command::run_visible(program, args),
        Escalator::Sudo => {
            let mut sudo_args = vec![program];
            sudo_args.extend_from_slice(args);
            command::run_visible("sudo", &sudo_args)
        }
        Escalator::Doas => {
            let mut doas_args = vec![program];
            doas_args.extend_from_slice(args);
            command::run_visible("doas", &doas_args)
        }
        Escalator::Su => {
            // su -c requires the full command as a single shell string
            let full_cmd = std::iter::once(program)
                .chain(args.iter().copied())
                .map(shell_escape)
                .collect::<Vec<_>>()
                .join(" ");
            command::run_visible("su", &["-", "root", "-c", &full_cmd])
        }
    }
}

/// Minimally shell-escape a single argument for use inside `su -c "..."`.
///
/// Wraps the argument in single quotes and escapes any single quotes within it.
fn shell_escape(s: &str) -> String {
    // Replace ' with '\'' (end quote, escaped quote, reopen quote)
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::shell_escape;

    #[test]
    fn escape_plain_string() {
        assert_eq!(shell_escape("hello"), "'hello'");
    }

    #[test]
    fn escape_string_with_spaces() {
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn escape_string_with_single_quote() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
    }

    #[test]
    fn escape_empty_string() {
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn escape_flags() {
        assert_eq!(shell_escape("-y"), "'-y'");
    }
}
