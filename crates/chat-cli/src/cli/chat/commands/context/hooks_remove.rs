use super::hooks_completion;
use crate::cli::chat::ChatError;
use crate::cli::chat::commands::{
    CommandHandler,
    CompletionContextAdapter,
};

/// Static instance of the remove hooks command handler
pub static REMOVE_HOOKS_HANDLER: RemoveHooksCommand = RemoveHooksCommand;

/// Handler for the context hooks remove command
pub struct RemoveHooksCommand;

impl CommandHandler for RemoveHooksCommand {
    fn name(&self) -> &'static str {
        "remove"
    }

    fn description(&self) -> &'static str {
        "Remove an existing context hook"
    }

    fn usage(&self) -> &'static str {
        "/context hooks rm [--global] <name>"
    }

    fn help(&self) -> String {
        "Remove an existing context hook. Use --global to remove from global hooks.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<crate::cli::chat::command::Command, ChatError> {
        let mut global = false;
        let mut name = None;

        for arg in args {
            if arg == "--global" {
                global = true;
            } else {
                name = Some(arg.to_string());
            }
        }

        // Validate required parameters
        let name = name.ok_or_else(|| ChatError::Custom("Hook name is required".into()))?;

        Ok(crate::cli::chat::command::Command::Context {
            subcommand: crate::cli::chat::command::ContextSubcommand::Hooks {
                subcommand: Some(crate::cli::chat::command::HooksSubcommand::Remove { name, global }),
            },
        })
    }

    fn complete_arguments(&self, args: &[&str], ctx: Option<&CompletionContextAdapter<'_>>) -> Vec<String> {
        // Check if we have the --global flag
        let is_global = args.contains(&"--global");

        // If we have a partial hook name, suggest matching hooks
        if let Some(ctx) = ctx {
            return hooks_completion::get_hook_name_suggestions(args, Some(ctx), is_global);
        }

        // If no context available, suggest the --global flag
        if !is_global {
            return vec!["--global".to_string()];
        }

        Vec::new()
    }
}
