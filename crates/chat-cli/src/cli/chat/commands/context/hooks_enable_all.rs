use crate::cli::chat::ChatError;
use crate::cli::chat::commands::{
    CommandHandler,
    CompletionContextAdapter,
};

/// Static instance of the enable-all hooks command handler
pub static ENABLE_ALL_HOOKS_HANDLER: EnableAllHooksCommand = EnableAllHooksCommand;

/// Handler for the context hooks enable-all command
pub struct EnableAllHooksCommand;

impl CommandHandler for EnableAllHooksCommand {
    fn name(&self) -> &'static str {
        "enable-all"
    }

    fn description(&self) -> &'static str {
        "Enable all existing context hooks"
    }

    fn usage(&self) -> &'static str {
        "/context hooks enable-all [--global]"
    }

    fn help(&self) -> String {
        "Enable all existing context hooks. Use --global to enable all global hooks.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<crate::cli::chat::command::Command, ChatError> {
        let global = args.contains(&"--global");

        Ok(crate::cli::chat::command::Command::Context {
            subcommand: crate::cli::chat::command::ContextSubcommand::Hooks {
                subcommand: Some(crate::cli::chat::command::HooksSubcommand::EnableAll { global }),
            },
        })
    }

    fn complete_arguments(&self, args: &[&str], _ctx: Option<&CompletionContextAdapter<'_>>) -> Vec<String> {
        // If we don't have the --global flag yet, suggest it
        if !args.contains(&"--global") {
            return vec!["--global".to_string()];
        }

        Vec::new()
    }
}
