use std::future::Future;
use std::io::Write;
use std::pin::Pin;

use crossterm::queue;
use crossterm::style::{
    self,
    Color,
};

use crate::cli::chat::commands::CommandHandler;
use crate::cli::chat::{
    ChatError,
    ChatState,
    QueuedTool,
};

/// Static instance of the add context command handler
pub static ADD_CONTEXT_HANDLER: AddContextCommand = AddContextCommand;

/// Handler for the context add command
pub struct AddContextCommand;

impl CommandHandler for AddContextCommand {
    fn name(&self) -> &'static str {
        "add"
    }

    fn description(&self) -> &'static str {
        "Add file(s) to context"
    }

    fn usage(&self) -> &'static str {
        "/context add [--global] [--force] <path1> [path2...]"
    }

    fn help(&self) -> String {
        "Add files to the context. Use --global to add to global context (available in all profiles). Use --force to add files even if they exceed size limits.".to_string()
    }

    fn to_command(&self, args: Vec<&str>) -> Result<crate::cli::chat::command::Command, ChatError> {
        let mut global = false;
        let mut force = false;
        let mut paths = Vec::new();

        for arg in args {
            match arg {
                "--global" => global = true,
                "--force" => force = true,
                _ => paths.push(arg.to_string()),
            }
        }

        Ok(crate::cli::chat::command::Command::Context {
            subcommand: crate::cli::chat::command::ContextSubcommand::Add { global, force, paths },
        })
    }

    fn execute_command<'a>(
        &'a self,
        command: &'a crate::cli::chat::command::Command,
        ctx: &'a mut crate::cli::chat::commands::context_adapter::CommandContextAdapter<'a>,
        tool_uses: Option<Vec<QueuedTool>>,
        pending_tool_index: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = Result<ChatState, ChatError>> + Send + 'a>> {
        Box::pin(async move {
            // Extract the parameters from the command
            let (global, force, paths) = match command {
                crate::cli::chat::command::Command::Context {
                    subcommand: crate::cli::chat::command::ContextSubcommand::Add { global, force, paths },
                } => (global, force, paths),
                _ => return Err(ChatError::Custom("Invalid command".into())),
            };

            // Check if paths are provided
            if paths.is_empty() {
                return Err(ChatError::Custom(
                    format!("No paths specified. Usage: {}", self.usage()).into(),
                ));
            }

            // Get the context manager
            let Some(context_manager) = &mut ctx.conversation_state.context_manager else {
                queue!(
                    ctx.output,
                    style::SetForegroundColor(Color::Red),
                    style::Print("Error: Context manager not initialized\n"),
                    style::ResetColor
                )?;
                ctx.output.flush()?;
                return Ok(ChatState::PromptUser {
                    tool_uses,
                    pending_tool_index,
                    skip_printing_tools: true,
                });
            };

            // Add the paths to the context
            match context_manager.add_paths(paths.clone(), *global, *force).await {
                Ok(_) => {
                    // Success message
                    let scope = if *global { "global" } else { "profile" };
                    queue!(
                        ctx.output,
                        style::SetForegroundColor(Color::Green),
                        style::Print(format!("Added {} file(s) to {} context\n", paths.len(), scope)),
                        style::ResetColor
                    )?;
                    ctx.output.flush()?;
                },
                Err(e) => {
                    // Error message
                    queue!(
                        ctx.output,
                        style::SetForegroundColor(Color::Red),
                        style::Print(format!("Error: {}\n", e)),
                        style::ResetColor
                    )?;
                    ctx.output.flush()?;
                },
            }

            // Return to prompt
            Ok(ChatState::PromptUser {
                tool_uses,
                pending_tool_index,
                skip_printing_tools: true,
            })
        })
    }

    fn requires_confirmation(&self, _args: &[&str]) -> bool {
        true // Adding context files requires confirmation as it's a mutative operation
    }

    fn complete_arguments(
        &self,
        args: &[&str],
        ctx: Option<&crate::cli::chat::commands::CompletionContextAdapter<'_>>,
    ) -> Vec<String> {
        use std::fs;
        use std::path::Path;

        // Filter out flags to get the last path argument if any
        let path_args: Vec<&str> = args
            .iter()
            .filter(|&&arg| arg != "--global" && arg != "--force")
            .copied()
            .collect();

        let mut completions = Vec::new();

        // If we have a path argument, use it for completion
        if let Some(last_path) = path_args.last() {
            let path = Path::new(last_path);

            // If the path is a directory that exists, list its contents
            if path.is_dir() {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.filter_map(Result::ok) {
                        let entry_path = entry.path();
                        let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

                        // Add trailing slash for directories
                        let suggestion = if entry_path.is_dir() {
                            format!("{}/{}", last_path.trim_end_matches('/'), file_name)
                        } else {
                            format!("{}/{}", last_path.trim_end_matches('/'), file_name)
                        };

                        completions.push(suggestion);
                    }
                }
            } else {
                // Try to complete based on the parent directory
                if let Some(parent) = path.parent() {
                    let file_prefix = path.file_name().unwrap_or_default().to_string_lossy();

                    if let Ok(entries) = fs::read_dir(parent) {
                        for entry in entries.filter_map(Result::ok) {
                            let entry_path = entry.path();
                            let file_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

                            if file_name.to_string().starts_with(&*file_prefix) {
                                let parent_str = parent.to_string_lossy();
                                let suggestion = if entry_path.is_dir() {
                                    format!("{}/{}/", parent_str, file_name)
                                } else {
                                    format!("{}/{}", parent_str, file_name)
                                };

                                completions.push(suggestion);
                            }
                        }
                    }
                }
            }
        } else {
            // No path argument yet, suggest common directories
            completions.extend(vec![
                "./".to_string(),
                "../".to_string(),
                "/".to_string(),
                "~/".to_string(),
            ]);
        }

        // If we have context manager, also suggest existing context files
        if let Some(ctx) = ctx {
            if let Some(_context_manager) = &ctx.conversation_state.context_manager {
                // We can't directly use get_context_files() here because it's async
                // and complete_arguments is synchronous. In a real implementation,
                // we would need to refactor the API to support async completions.
                // For now, we'll just add some common context files as suggestions.
                let common_context_files = vec![
                    "README.md".to_string(),
                    "CONTRIBUTING.md".to_string(),
                    "LICENSE.md".to_string(),
                ];

                for file_path in common_context_files {
                    // Only add if it's not already in the list and matches the prefix
                    if let Some(last_path) = path_args.last() {
                        if file_path.starts_with(last_path) && !completions.contains(&file_path) {
                            completions.push(file_path);
                        }
                    } else if !completions.contains(&file_path) {
                        completions.push(file_path);
                    }
                }
            }
        }

        completions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::chat::command::{
        Command,
        ContextSubcommand,
    };

    #[test]
    fn test_to_command_with_global_and_force() {
        let handler = AddContextCommand;
        let args = vec!["--global", "--force", "path1", "path2"];

        let command = handler.to_command(args).unwrap();

        match command {
            Command::Context {
                subcommand: ContextSubcommand::Add { global, force, paths },
            } => {
                assert!(global);
                assert!(force);
                assert_eq!(paths, vec!["path1".to_string(), "path2".to_string()]);
            },
            _ => panic!("Expected Context Add command"),
        }
    }

    #[test]
    fn test_to_command_with_global_only() {
        let handler = AddContextCommand;
        let args = vec!["--global", "path1", "path2"];

        let command = handler.to_command(args).unwrap();

        match command {
            Command::Context {
                subcommand: ContextSubcommand::Add { global, force, paths },
            } => {
                assert!(global);
                assert!(!force);
                assert_eq!(paths, vec!["path1".to_string(), "path2".to_string()]);
            },
            _ => panic!("Expected Context Add command"),
        }
    }

    #[test]
    fn test_to_command_with_force_only() {
        let handler = AddContextCommand;
        let args = vec!["--force", "path1", "path2"];

        let command = handler.to_command(args).unwrap();

        match command {
            Command::Context {
                subcommand: ContextSubcommand::Add { global, force, paths },
            } => {
                assert!(!global);
                assert!(force);
                assert_eq!(paths, vec!["path1".to_string(), "path2".to_string()]);
            },
            _ => panic!("Expected Context Add command"),
        }
    }
}
