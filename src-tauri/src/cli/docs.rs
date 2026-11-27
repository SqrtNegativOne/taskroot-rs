//! Renders `docs/cli.md` directly from the `clap` command definition.

#[cfg(test)]
use std::fmt::Write;

#[cfg(test)]
use clap::{Arg, ArgAction, Command, CommandFactory};

#[cfg(test)]
use super::spec::{Cli, HELP_TAIL};

#[cfg(test)]
#[must_use]
pub fn render_markdown() -> String {
    let root = Cli::command();
    let mut text = String::from(
        "# taskroot CLI\n\n\
         <!-- Generated from the clap CLI definition in src-tauri/src/cli/spec.rs \
         by `cargo nextest run`. Do not edit. -->\n\n\
         Headless Taskroot for agents and scripts: JSON on stdout, `AppError` text on\n\
         stderr with a non-zero exit code.\n\n\
         ```\nbun run cli -- <command>\n```\n\n\
         ## Commands\n\n| Command | Does |\n| --- | --- |\n",
    );

    for sub in root.get_subcommands() {
        let usage = subcommand_usage(sub);
        let about = sub.get_about().map_or_default(ToString::to_string);
        let _ = writeln!(text, "| `{usage}` | {about} |");
    }

    text.push_str("\n## Command options\n");
    for sub in root.get_subcommands() {
        let options: Vec<&Arg> = sub
            .get_arguments()
            .filter(|arg| arg.get_long().is_some())
            .collect();
        if options.is_empty() {
            continue;
        }

        let _ = writeln!(text, "\n### `{}`\n", sub.get_name());
        text.push_str("| Flag | Meaning |\n| --- | --- |\n");
        for arg in options {
            let flag = format_flag(arg);
            let meaning = format_help(arg);
            let _ = writeln!(text, "| `{flag}` | {meaning} |");
        }
    }

    text.push('\n');
    text.push_str(HELP_TAIL);
    text.push('\n');
    text
}

#[cfg(test)]
fn subcommand_usage(sub: &Command) -> String {
    let mut parts = vec![sub.get_name().to_string()];

    for arg in sub.get_positionals() {
        let name = arg
            .get_value_names()
            .and_then(|names| names.first())
            .map_or_else(
                || arg.get_id().as_str().to_lowercase(),
                |name| name.as_str().to_lowercase(),
            );
        let suffix = if matches!(arg.get_action(), ArgAction::Append) {
            "..."
        } else {
            ""
        };
        if arg.is_required_set() {
            parts.push(format!("<{name}>{suffix}"));
        } else {
            parts.push(format!("[{name}]{suffix}"));
        }
    }

    let mut has_optional_flag = false;
    for arg in sub.get_arguments().filter(|arg| arg.get_long().is_some()) {
        if arg.is_required_set() {
            parts.push(format_flag(arg));
        } else {
            has_optional_flag = true;
        }
    }
    if has_optional_flag {
        parts.push("[options]".to_string());
    }

    parts.join(" ")
}

#[cfg(test)]
fn format_flag(arg: &Arg) -> String {
    let long = arg.get_long().unwrap_or_default();
    if !arg.get_action().takes_values() {
        return format!("--{long}");
    }
    let value = arg
        .get_value_names()
        .and_then(|names| names.first())
        .map_or_else(
            || long.to_uppercase(),
            |name| name.as_str().to_string(),
        );
    format!("--{long} <{value}>")
}

#[cfg(test)]
fn format_help(arg: &Arg) -> String {
    let mut help = arg.get_help().map_or_default(ToString::to_string);
    if arg.get_action().takes_values() {
        let values: Vec<String> = arg
            .get_possible_values()
            .into_iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_string())
            .collect();
        if !values.is_empty() {
            let joined = values.join(", ");
            let _ = write!(help, " (values: {joined})");
        }
    }
    if arg.is_required_set() {
        help.push_str(" *(required)*");
    }
    help
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_the_cli_reference() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/cli.md");
        std::fs::write(path, render_markdown()).unwrap();
    }
}
