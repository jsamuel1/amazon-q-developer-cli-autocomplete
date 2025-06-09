use crate::cli::chat::ChatError;
use crate::cli::chat::commands::{
    CommandHandler,
    CompletionContextAdapter,
};

/// Static instance of the disable-all hooks command handler
pub static DISABLE_ALL_HOOKS_HANDLER: DisableAllHooksCommand = DisableAllHooksCommand;

/// Handler for the context hooks disable-all command
pub struct DisableAllHooksCommand;

impl CommandHandler for DisableAllHooksCommand {
    fn name(&self) -> &'static str {
        "disable-all"
    }

    fn description(&self) -> &'static str {
        "Disable all existing context hooks"
    }

    fn usage(&self) -> &'static str {
        "/context hooks disable-all [--global]"
    }

    fn help(&self) -> String {
        "Disable all existing context hooks. Use --global to disable all global hooks.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<crate::cli::chat::command::Command, ChatError> {
        let global = args.contains(&"--global");

        Ok(crate::cli::chat::command::Command::Context {
            subcommand: crate::cli::chat::command::ContextSubcommand::Hooks {
                subcommand: Some(crate::cli::chat::command::HooksSubcommand::DisableAll { global }),
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
