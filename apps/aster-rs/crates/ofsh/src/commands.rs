use crate::registry::{CommandRegistry, CommandResult, CommandSpec};

pub fn builtin_registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();

    registry.register(CommandSpec::new("help", "Show help"), |_args| {
        CommandResult::output(help_text())
    });
    registry.register(
        CommandSpec::new("echo", "Output text with newline"),
        |args| CommandResult::output(format!("{}\n", args.join(" "))),
    );
    registry.register(CommandSpec::new("exit", "Exit shell"), |_args| {
        CommandResult::exit()
    });
    registry.register(CommandSpec::new("quit", "Exit shell"), |_args| {
        CommandResult::exit()
    });

    registry
}

fn help_text() -> String {
    [
        "ofsh - ObolosFS Shell",
        "",
        "Commands:",
        "  help                Show this help",
        "  echo <text...>      Output text with newline",
        "  exit                Exit shell",
        "  quit                Exit shell",
        "",
        "VFS, pipes, and redirection are planned but not supported by this Rust v1.",
    ]
    .join("\n")
}
