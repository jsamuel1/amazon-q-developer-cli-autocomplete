pub mod add;
pub mod clear;
pub mod hooks_add;
pub mod hooks_completion;
pub mod hooks_disable;
pub mod hooks_disable_all;
pub mod hooks_enable;
pub mod hooks_enable_all;
pub mod hooks_help;
pub mod hooks_remove;
pub mod remove;
pub mod show;

use self::hooks_add::ADD_HOOKS_HANDLER;
use self::hooks_disable::DISABLE_HOOKS_HANDLER;
use self::hooks_disable_all::DISABLE_ALL_HOOKS_HANDLER;
use self::hooks_enable::ENABLE_HOOKS_HANDLER;
use self::hooks_enable_all::ENABLE_ALL_HOOKS_HANDLER;
use self::hooks_help::HOOKS_HELP_HANDLER;
use self::hooks_remove::REMOVE_HOOKS_HANDLER;
use crate::cli::chat::ChatError;
use crate::cli::chat::command::{
    Command,
    ContextSubcommand,
};
use crate::cli::chat::commands::CompletionContextAdapter;
use crate::cli::chat::commands::handler::CommandHandler;

/// Static instance of the context command handler
pub static CONTEXT_HANDLER: ContextCommand = ContextCommand;

/// Handler for the context command
pub struct ContextCommand;

impl CommandHandler for ContextCommand {
    fn name(&self) -> &'static str {
        "context"
    }

    fn description(&self) -> &'static str {
        "Manage context files and hooks for the chat session"
    }

    fn usage(&self) -> &'static str {
        "/context [subcommand]"
    }

    fn help(&self) -> String {
        ContextSubcommand::help_text()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<Command, ChatError> {
        if args.is_empty() {
            return Ok(Command::Context {
                subcommand: ContextSubcommand::Show { expand: false },
            });
        }

        match args[0] {
            "help" => Ok(Command::Context {
                subcommand: ContextSubcommand::Help,
            }),
            "hooks" => {
                if args.len() == 1 {
                    return Ok(Command::Context {
                        subcommand: ContextSubcommand::Hooks { subcommand: None },
                    });
                }

                let subcommand = match args[1] {
                    "help" => HOOKS_HELP_HANDLER.to_command(args[2..].to_vec())?,
                    "add" => ADD_HOOKS_HANDLER.to_command(args[2..].to_vec())?,
                    "rm" => REMOVE_HOOKS_HANDLER.to_command(args[2..].to_vec())?,
                    "enable" => ENABLE_HOOKS_HANDLER.to_command(args[2..].to_vec())?,
                    "disable" => DISABLE_HOOKS_HANDLER.to_command(args[2..].to_vec())?,
                    "enable-all" => ENABLE_ALL_HOOKS_HANDLER.to_command(args[2..].to_vec())?,
                    "disable-all" => DISABLE_ALL_HOOKS_HANDLER.to_command(args[2..].to_vec())?,
                    _ => {
                        return Err(ChatError::Custom(
                            format!("Unknown hooks subcommand: {}", args[1]).into(),
                        ));
                    },
                };

                Ok(subcommand)
            },
            _ => Err(ChatError::Custom(
                format!("Unknown context subcommand: {}", args[0]).into(),
            )),
        }
    }

    fn complete_arguments(&self, args: &[&str], ctx: Option<&CompletionContextAdapter<'_>>) -> Vec<String> {
        if args.is_empty() {
            // Suggest all context subcommands
            return vec![
                "show".to_string(),
                "add".to_string(),
                "rm".to_string(),
                "clear".to_string(),
                "hooks".to_string(),
                "help".to_string(),
            ];
        }

        // If we have a subcommand, delegate to the appropriate handler
        match args[0] {
            "hooks" => {
                if args.len() == 1 {
                    // Suggest all hooks subcommands
                    return vec![
                        "help".to_string(),
                        "add".to_string(),
                        "rm".to_string(),
                        "enable".to_string(),
                        "disable".to_string(),
                        "enable-all".to_string(),
                        "disable-all".to_string(),
                    ];
                }

                // Delegate to the appropriate hooks subcommand handler
                match args[1] {
                    "help" => HOOKS_HELP_HANDLER.complete_arguments(&args[2..], ctx),
                    "add" => ADD_HOOKS_HANDLER.complete_arguments(&args[2..], ctx),
                    "rm" => REMOVE_HOOKS_HANDLER.complete_arguments(&args[2..], ctx),
                    "enable" => ENABLE_HOOKS_HANDLER.complete_arguments(&args[2..], ctx),
                    "disable" => DISABLE_HOOKS_HANDLER.complete_arguments(&args[2..], ctx),
                    "enable-all" => ENABLE_ALL_HOOKS_HANDLER.complete_arguments(&args[2..], ctx),
                    "disable-all" => DISABLE_ALL_HOOKS_HANDLER.complete_arguments(&args[2..], ctx),
                    _ => Vec::new(),
                }
            },
            _ => Vec::new(),
        }
    }
}
