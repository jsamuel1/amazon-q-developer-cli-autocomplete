use std::future::Future;
use std::io::Write;
use std::pin::Pin;

use crossterm::queue;
use crossterm::style::{
    self,
    Color,
};

use crate::cli::chat::command::{
    Command,
    ProfileSubcommand,
};
use crate::cli::chat::commands::context_adapter::CommandContextAdapter;
use crate::cli::chat::commands::handler::CommandHandler;
use crate::cli::chat::{
    ChatError,
    ChatState,
    QueuedTool,
};

/// Static instance of the profile set command handler
pub static SET_PROFILE_HANDLER: SetProfileCommand = SetProfileCommand;

/// Handler for the profile set command
pub struct SetProfileCommand;

impl CommandHandler for SetProfileCommand {
    fn name(&self) -> &'static str {
        "set"
    }

    fn description(&self) -> &'static str {
        "Set the current profile"
    }

    fn usage(&self) -> &'static str {
        "/profile set <n>"
    }

    fn help(&self) -> String {
        "Switch to the specified profile.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<Command, ChatError> {
        if args.len() != 1 {
            return Err(ChatError::Custom("Expected profile name argument".into()));
        }

        Ok(Command::Profile {
            subcommand: ProfileSubcommand::Set {
                name: args[0].to_string(),
            },
        })
    }

    fn execute_command<'a>(
        &'a self,
        command: &'a Command,
        ctx: &'a mut CommandContextAdapter<'a>,
        tool_uses: Option<Vec<QueuedTool>>,
        pending_tool_index: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatState, ChatError>> + Send + 'a>> {
        Box::pin(async move {
            // Extract the profile name from the command
            let name = match command {
                Command::Profile {
                    subcommand: ProfileSubcommand::Set { name },
                } => name,
                _ => return Err(ChatError::Custom("Invalid command".into())),
            };

            // Get the context manager
            if let Some(context_manager) = &mut ctx.conversation_state.context_manager {
                // Switch to the profile
                match context_manager.switch_profile(name).await {
                    Ok(_) => {
                        queue!(
                            ctx.output,
                            style::Print("\nSwitched to profile '"),
                            style::SetForegroundColor(Color::Green),
                            style::Print(name),
                            style::ResetColor,
                            style::Print("'.\n\n")
                        )?;
                    },
                    Err(e) => {
                        queue!(
                            ctx.output,
                            style::SetForegroundColor(Color::Red),
                            style::Print(format!("\nError switching profile: {}\n\n", e)),
                            style::ResetColor
                        )?;
                    },
                }
                ctx.output.flush()?;
            } else {
                queue!(
                    ctx.output,
                    style::SetForegroundColor(Color::Red),
                    style::Print("\nContext manager is not available.\n\n"),
                    style::ResetColor
                )?;
                ctx.output.flush()?;
            }

            Ok(ChatState::PromptUser {
                tool_uses,
                pending_tool_index,
                skip_printing_tools: true,
            })
        })
    }

    fn requires_confirmation(&self, _args: &[&str]) -> bool {
        true // Set command requires confirmation as it's a mutative operation
    }

    fn complete_arguments(
        &self,
        args: &[&str],
        ctx: Option<&crate::cli::chat::commands::CompletionContextAdapter<'_>>,
    ) -> Vec<String> {
        if let Some(ctx) = ctx {
            // If we have a completion cache, use it for better suggestions
            if ctx.completion_cache.has_category("profiles") {
                if let Some(partial_input) = args.last() {
                    // Use fuzzy matching for better suggestions
                    return ctx
                        .completion_cache
                        .get_best_matches("profiles", "all", partial_input, 10);
                } else {
                    // Return all profiles
                    return ctx.completion_cache.get("profiles", "all");
                }
            }

            // Fallback to direct context manager access if cache is not available
            if let Some(context_manager) = &ctx.conversation_state.context_manager {
                if let Ok(profiles) = context_manager.list_profiles_blocking() {
                    return profiles;
                }
            }
        }

        // Default: no suggestions
        Vec::new()
    }
}
