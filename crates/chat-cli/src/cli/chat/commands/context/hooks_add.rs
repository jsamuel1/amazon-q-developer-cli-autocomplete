use super::hooks_completion;
use crate::cli::chat::ChatError;
use crate::cli::chat::commands::{
    CommandHandler,
    CompletionContextAdapter,
};

/// Static instance of the add hooks command handler
pub static ADD_HOOKS_HANDLER: AddHooksCommand = AddHooksCommand;

/// Handler for the context hooks add command
pub struct AddHooksCommand;

impl CommandHandler for AddHooksCommand {
    fn name(&self) -> &'static str {
        "add"
    }

    fn description(&self) -> &'static str {
        "Add a new command context hook"
    }

    fn usage(&self) -> &'static str {
        "/context hooks add [--global] <n> --trigger <trigger> --command <command>"
    }

    fn help(&self) -> String {
        "Add a new command context hook. Use --global to add to global hooks.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<crate::cli::chat::command::Command, ChatError> {
        let mut global = false;
        let mut name = None;
        let mut trigger = None;
        let mut command = None;

        let mut i = 0;
        while i < args.len() {
            match args[i] {
                "--global" => {
                    global = true;
                    i += 1;
                },
                "--trigger" => {
                    if i + 1 < args.len() {
                        trigger = Some(args[i + 1].to_string());
                        i += 2;
                    } else {
                        return Err(ChatError::Custom("Missing trigger value".into()));
                    }
                },
                "--command" => {
                    if i + 1 < args.len() {
                        command = Some(args[i + 1].to_string());
                        i += 2;
                    } else {
                        return Err(ChatError::Custom("Missing command value".into()));
                    }
                },
                arg if arg.starts_with("--trigger=") => {
                    trigger = Some(arg.strip_prefix("--trigger=").unwrap().to_string());
                    i += 1;
                },
                arg if arg.starts_with("--command=") => {
                    command = Some(arg.strip_prefix("--command=").unwrap().to_string());
                    i += 1;
                },
                arg => {
                    if name.is_none() {
                        name = Some(arg.to_string());
                    }
                    i += 1;
                },
            }
        }

        // Validate required parameters
        let name = name.ok_or_else(|| ChatError::Custom("Hook name is required".into()))?;
        let trigger = trigger.ok_or_else(|| ChatError::Custom("Trigger is required (--trigger)".into()))?;
        let command = command.ok_or_else(|| ChatError::Custom("Command is required (--command)".into()))?;

        // Create hook command
        Ok(crate::cli::chat::command::Command::Context {
            subcommand: crate::cli::chat::command::ContextSubcommand::Hooks {
                subcommand: Some(crate::cli::chat::command::HooksSubcommand::Add {
                    name,
                    trigger,
                    command,
                    global,
                }),
            },
        })
    }

    fn complete_arguments(&self, args: &[&str], ctx: Option<&CompletionContextAdapter<'_>>) -> Vec<String> {
        // Check if we're after a specific flag
        if args.len() > 1 {
            if let Some(last_arg) = args.last() {
                if last_arg == &"--trigger" {
                    return vec!["per_prompt".to_string(), "conversation_start".to_string()];
                }

                // Handle partial trigger values
                if last_arg.starts_with("--trigger=") {
                    return hooks_completion::get_trigger_suggestions(args);
                }
                
                // Handle command completion using path completer
                if last_arg == &"--command" {
                    if let Some(ctx) = ctx {
                        // Use the path completer if available
                        if let Some(path_completer) = &ctx.path_completer {
                            // Get file suggestions
                            return path_completer.complete("");
                        }
                    }
                }

                // Handle partial command values with path completion
                if last_arg.starts_with("--command=") {
                    if let Some(ctx) = ctx {
                        if let Some(path_completer) = &ctx.path_completer {
                            let path_prefix = last_arg.strip_prefix("--command=").unwrap_or("");
                            return path_completer.complete(path_prefix);
                        }
                    }
                }
            }
        }

        // If no specific completion needed, suggest flags
        vec!["--global".to_string(), "--trigger".to_string(), "--command".to_string()]
    }
}
