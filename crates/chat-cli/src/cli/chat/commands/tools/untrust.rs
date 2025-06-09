use std::collections::HashSet;
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

/// Static instance of the tools untrust command handler
pub static UNTRUST_TOOLS_HANDLER: UntrustToolsCommand = UntrustToolsCommand;

/// Handler for the tools untrust command
pub struct UntrustToolsCommand;
impl CommandHandler for UntrustToolsCommand {
    fn name(&self) -> &'static str {
        "untrust"
    }

    fn description(&self) -> &'static str {
        "Revert a tool to per-request confirmation"
    }

    fn usage(&self) -> &'static str {
        "/tools untrust <tool_name> [tool_name...]"
    }

    fn help(&self) -> String {
        "Untrust specific tools, reverting them to per-request confirmation.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<Command, ChatError> {
        if args.is_empty() {
            return Err(ChatError::Custom("Expected at least one tool name".into()));
        }

        let tool_names: HashSet<String> = args.iter().map(|s| (*s).to_string()).collect();
        Ok(Command::Tools {
            subcommand: Some(ToolsSubcommand::Untrust { tool_names }),
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
            // Extract the tool names from the command
            let tool_names = match command {
                Command::Tools {
                    subcommand: Some(ToolsSubcommand::Untrust { tool_names }),
                } => tool_names,
                _ => {
                    return Err(ChatError::Custom(
                        "UntrustToolsCommand can only execute Untrust commands".into(),
                    ));
                },
            };

            // Untrust the specified tools
            for tool_name in tool_names {
                // Check if the tool exists in the conversation state's tools
                let tool_exists = ctx.conversation_state.tools.values().any(|tools| {
                    tools.iter().any(|tool| match tool {
                        crate::api_client::model::Tool::ToolSpecification(spec) => &spec.name == tool_name,
                    })
                });

                if !tool_exists {
                    queue!(
                        ctx.output,
                        style::SetForegroundColor(Color::Red),
                        style::Print(format!("\nUnknown tool: '{}'\n", tool_name)),
                        style::ResetColor
                    )?;
                    continue;
                }

                // Untrust the tool
                ctx.tool_permissions.untrust_tool(tool_name);

                queue!(
                    ctx.output,
                    style::SetForegroundColor(Color::Green),
                    style::Print(format!("\nTool '{}' is set to per-request confirmation.\n", tool_name)),
                    style::ResetColor
                )?;
            }

            queue!(ctx.output, style::Print("\n"))?;
            ctx.output.flush()?;

            Ok(ChatState::PromptUser {
                tool_uses,
                pending_tool_index,
                skip_printing_tools: true,
            })
        })
    }

    fn requires_confirmation(&self, _args: &[&str]) -> bool {
        true // Untrust command requires confirmation as it's a mutative operation
    }

    fn complete_arguments(
        &self,
        args: &[&str],
        ctx: Option<&crate::cli::chat::commands::CompletionContextAdapter<'_>>,
    ) -> Vec<String> {
        if let Some(ctx) = ctx {
            // If we have a completion cache, use it for better suggestions
            if ctx.completion_cache.has_category("tools") {
                if let Some(partial_input) = args.last() {
                    // Use fuzzy matching for better suggestions
                    return ctx
                        .completion_cache
                        .get_best_matches("tools", "trusted", partial_input, 10);
                } else {
                    // Return all trusted tools
                    return ctx.completion_cache.get("tools", "trusted");
                }
            }

            // Fallback to direct tool manager access if cache is not available
            return crate::cli::chat::tool_manager::ToolManager::get_filtered_tool_names(
                ctx.conversation_state,
                ctx.tool_permissions,
                true, // We want trusted tools for the untrust command
            );
        }

        // Default: no suggestions
        Vec::new()
    }
}
