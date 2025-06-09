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
    ToolsSubcommand,
};
use crate::cli::chat::commands::context_adapter::CommandContextAdapter;
use crate::cli::chat::commands::handler::CommandHandler;
use crate::cli::chat::{
    ChatError,
    ChatState,
    QueuedTool,
};

/// Static instance of the tools reset single command handler
pub static RESET_SINGLE_TOOL_HANDLER: ResetSingleToolCommand = ResetSingleToolCommand;

/// Handler for the tools reset single command
pub struct ResetSingleToolCommand;
impl CommandHandler for ResetSingleToolCommand {
    fn name(&self) -> &'static str {
        "reset"
    }

    fn description(&self) -> &'static str {
        "Reset a specific tool to default permission level"
    }

    fn usage(&self) -> &'static str {
        "/tools reset <tool_name>"
    }

    fn help(&self) -> String {
        "Reset a specific tool to its default permission level.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<Command, ChatError> {
        if args.len() != 1 {
            return Err(ChatError::Custom("Expected tool name argument".into()));
        }

        Ok(Command::Tools {
            subcommand: Some(ToolsSubcommand::ResetSingle {
                tool_name: args[0].to_string(),
            }),
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
            // Extract the tool name from the command
            let tool_name = match command {
                Command::Tools {
                    subcommand: Some(ToolsSubcommand::ResetSingle { tool_name }),
                } => tool_name,
                _ => {
                    return Err(ChatError::Custom(
                        "ResetSingleToolCommand can only execute ResetSingle commands".into(),
                    ));
                },
            };

            // Check if the tool exists
            if !ctx.tool_permissions.has(tool_name) {
                queue!(
                    ctx.output,
                    style::SetForegroundColor(Color::Red),
                    style::Print(format!("\nUnknown tool: '{}'\n\n", tool_name)),
                    style::ResetColor
                )?;
            } else {
                // Reset the tool permission
                ctx.tool_permissions.reset_tool(tool_name);

                queue!(
                    ctx.output,
                    style::SetForegroundColor(Color::Green),
                    style::Print(format!("\nReset tool '{}' to default permission level.\n\n", tool_name)),
                    style::ResetColor
                )?;
            }
            ctx.output.flush()?;

            Ok(ChatState::PromptUser {
                tool_uses,
                pending_tool_index,
                skip_printing_tools: true,
            })
        })
    }

    fn requires_confirmation(&self, _args: &[&str]) -> bool {
        true // Reset single command requires confirmation as it's a mutative operation
    }

    fn complete_arguments(
        &self,
        _args: &[&str],
        ctx: Option<&crate::cli::chat::commands::CompletionContextAdapter<'_>>,
    ) -> Vec<String> {
        // If we have context, suggest all tools
        if let Some(ctx) = ctx {
            // Get all tool names from the conversation state
            let mut tool_names = Vec::new();

            for tools in ctx.conversation_state.tools.values() {
                for tool in tools {
                    // Use a match statement instead of if let to avoid the irrefutable pattern warning
                    match tool {
                        crate::api_client::model::Tool::ToolSpecification(spec) => {
                            tool_names.push(spec.name.clone());
                        },
                    }
                }
            }

            // Sort for consistent presentation
            tool_names.sort();
            return tool_names;
        }

        // Default: no suggestions
        Vec::new()
    }
}
