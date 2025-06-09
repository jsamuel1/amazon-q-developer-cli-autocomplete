use crate::cli::chat::ChatError;
use crate::cli::chat::commands::{
    CommandHandler,
    CompletionContextAdapter,
};

/// Static instance of the hooks help command handler
pub static HOOKS_HELP_HANDLER: HooksHelpCommand = HooksHelpCommand;

/// Handler for the context hooks help command
pub struct HooksHelpCommand;

impl CommandHandler for HooksHelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn description(&self) -> &'static str {
        "Show help for context hooks commands"
    }

    fn usage(&self) -> &'static str {
        "/context hooks help"
    }

    fn help(&self) -> String {
        "Show help information for context hooks commands.".to_string()
    }

    fn to_command(&self, _args: Vec<&str>) -> Result<crate::cli::chat::command::Command, ChatError> {
        Ok(crate::cli::chat::command::Command::Context {
            subcommand: crate::cli::chat::command::ContextSubcommand::Hooks {
                subcommand: Some(crate::cli::chat::command::HooksSubcommand::Help),
            },
        })
    }

    fn complete_arguments(&self, _args: &[&str], _ctx: Option<&CompletionContextAdapter<'_>>) -> Vec<String> {
        // No arguments needed for help
        Vec::new()
    }
}
