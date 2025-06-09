use super::hooks_completion;
use crate::cli::chat::ChatError;
use crate::cli::chat::commands::{
    CommandHandler,
    CompletionContextAdapter,
};

/// Static instance of the enable hooks command handler
pub static ENABLE_HOOKS_HANDLER: EnableHooksCommand = EnableHooksCommand;

/// Handler for the context hooks enable command
pub struct EnableHooksCommand;

impl CommandHandler for EnableHooksCommand {
    fn name(&self) -> &'static str {
        "enable"
    }

    fn description(&self) -> &'static str {
        "Enable an existing context hook"
    }

    fn usage(&self) -> &'static str {
        "/context hooks enable [--global] <name>"
    }

    fn help(&self) -> String {
        "Enable an existing context hook. Use --global to enable a global hook.".to_string()
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
                subcommand: Some(crate::cli::chat::command::HooksSubcommand::Enable { name, global }),
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
