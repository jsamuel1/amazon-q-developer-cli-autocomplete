use crate::cli::chat::commands::CompletionContextAdapter;

/// Get hook name suggestions for context hooks commands
///
/// This function provides completion suggestions for hook names based on the current context
/// and whether we're looking for global or profile hooks.
///
/// # Arguments
///
/// * `args` - The current command arguments
/// * `ctx` - The completion context adapter
/// * `is_global` - Whether to suggest global hooks or profile hooks
///
/// # Returns
///
/// A vector of hook name suggestions
pub fn get_hook_name_suggestions(
    args: &[&str],
    ctx: Option<&CompletionContextAdapter<'_>>,
    is_global: bool,
) -> Vec<String> {
    if let Some(ctx) = ctx {
        if let Some(context_manager) = &ctx.conversation_state.context_manager {
            // Determine which hooks to use based on the global flag
            let hooks = if is_global {
                &context_manager.global_config.hooks
            } else {
                &context_manager.profile_config.hooks
            };

            // If we have a partial hook name, filter by it
            if let Some(partial_name) = args.last() {
                if partial_name != &"--global" {
                    return hooks
                        .keys()
                        .filter(|name| name.starts_with(partial_name))
                        .cloned()
                        .collect();
                }
            }

            // Otherwise return all hook names
            return hooks.keys().cloned().collect();
        }
    }

    Vec::new()
}

/// Get trigger type suggestions for context hooks add command
///
/// # Returns
///
/// A vector of trigger type suggestions
pub fn get_trigger_suggestions(args: &[&str]) -> Vec<String> {
    let triggers = vec!["per_prompt".to_string(), "conversation_start".to_string()];

    if let Some(partial) = args.iter().find(|&&arg| arg.starts_with("--trigger=")) {
        let prefix = partial.strip_prefix("--trigger=").unwrap_or("");
        triggers
            .into_iter()
            .filter(|trigger| trigger.starts_with(prefix))
            .collect()
    } else {
        triggers
    }
}
