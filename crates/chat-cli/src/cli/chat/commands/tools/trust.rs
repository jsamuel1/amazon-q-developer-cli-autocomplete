use std::collections::HashSet;
use std::future::Future;
use std::io::Write;
use std::pin::Pin;

use crossterm::queue;
use crossterm::style::{
    self,
    Attribute,
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

/// Static instance of the tools trust command handler
pub static TRUST_TOOLS_HANDLER: TrustToolsCommand = TrustToolsCommand;

/// Handler for the tools trust command
pub struct TrustToolsCommand;

impl CommandHandler for TrustToolsCommand {
    fn name(&self) -> &'static str {
        "trust"
    }

    fn description(&self) -> &'static str {
        "Trust a specific tool for the session"
    }

    fn usage(&self) -> &'static str {
        "/tools trust <tool_name> [tool_name...]"
    }

    fn help(&self) -> String {
        "Trust specific tools for the session. Trusted tools will not require confirmation before running.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<Command, ChatError> {
        if args.is_empty() {
            return Err(ChatError::Custom("Expected at least one tool name".into()));
        }

        let tool_names: HashSet<String> = args.iter().map(|s| (*s).to_string()).collect();
        Ok(Command::Tools {
            subcommand: Some(ToolsSubcommand::Trust { tool_names }),
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
                    subcommand: Some(ToolsSubcommand::Trust { tool_names }),
                } => tool_names,
                _ => {
                    return Err(ChatError::Custom(
                        "TrustToolsCommand can only execute Trust commands".into(),
                    ));
                },
            };

            // Trust the specified tools
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

                // Trust the tool
                ctx.tool_permissions.trust_tool(tool_name);

                queue!(
                    ctx.output,
                    style::SetForegroundColor(Color::Green),
                    style::Print(format!("\nTool '{}' is now trusted. I will ", tool_name)),
                    style::SetAttribute(Attribute::Bold),
                    style::Print("not"),
                    style::SetAttribute(Attribute::NoBold),
                    style::Print(" ask for confirmation before running this tool.\n"),
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
        true // Trust command requires confirmation as it's a mutative operation
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
                        .get_best_matches("tools", "untrusted", partial_input, 10);
                } else {
                    // Return all untrusted tools
                    return ctx.completion_cache.get("tools", "untrusted");
                }
            }

            // Fallback to direct tool manager access if cache is not available
            return crate::cli::chat::tool_manager::ToolManager::get_filtered_tool_names(
                ctx.conversation_state,
                ctx.tool_permissions,
                false, // We want untrusted tools for the trust command
            );
        }

        // Default: no suggestions
        Vec::new()
    }
}
