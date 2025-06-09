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

/// Static instance of the profile delete command handler
pub static DELETE_PROFILE_HANDLER: DeleteProfileCommand = DeleteProfileCommand;

/// Handler for the profile delete command
pub struct DeleteProfileCommand;

impl CommandHandler for DeleteProfileCommand {
    fn name(&self) -> &'static str {
        "delete"
    }

    fn description(&self) -> &'static str {
        "Delete a profile"
    }

    fn usage(&self) -> &'static str {
        "/profile delete <n>"
    }

    fn help(&self) -> String {
        "Delete the specified profile.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<Command, ChatError> {
        if args.len() != 1 {
            return Err(ChatError::Custom("Expected profile name argument".into()));
        }

        Ok(Command::Profile {
            subcommand: ProfileSubcommand::Delete {
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
                    subcommand: ProfileSubcommand::Delete { name },
                } => name,
                _ => return Err(ChatError::Custom("Invalid command".into())),
            };

            // Get the context manager
            if let Some(context_manager) = &ctx.conversation_state.context_manager {
                // Delete the profile
                match context_manager.delete_profile(name).await {
                    Ok(_) => {
                        queue!(
                            ctx.output,
                            style::Print("\nProfile '"),
                            style::SetForegroundColor(Color::Green),
                            style::Print(name),
                            style::ResetColor,
                            style::Print("' deleted successfully.\n\n")
                        )?;
                    },
                    Err(e) => {
                        queue!(
                            ctx.output,
                            style::SetForegroundColor(Color::Red),
                            style::Print(format!("\nError deleting profile: {}\n\n", e)),
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
        true // Delete command requires confirmation
    }

    fn complete_arguments(
        &self,
        args: &[&str],
        ctx: Option<&crate::cli::chat::commands::CompletionContextAdapter<'_>>,
    ) -> Vec<String> {
        // If we have context and no arguments yet, suggest profile names
        if args.is_empty() {
            if let Some(ctx) = ctx {
                if let Some(context_manager) = &ctx.conversation_state.context_manager {
                    // Use the blocking version since we're in a synchronous context
                    if let Ok(profiles) = context_manager.list_profiles_blocking() {
                        // Filter out the default profile (can't be deleted) and the current profile
                        return profiles
                            .into_iter()
                            .filter(|p| {
                                p != "default" && Some(p.as_str()) != Some(context_manager.current_profile.as_str())
                            })
                            .collect();
                    }
                }
            }
        }

        // Default: no suggestions
        Vec::new()
    }
}
