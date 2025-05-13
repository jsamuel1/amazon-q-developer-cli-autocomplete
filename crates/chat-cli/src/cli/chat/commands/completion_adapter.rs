use crate::cli::chat::{
    ConversationState,
    ToolPermissions,
};

// Forward declaration of CompletionCache
pub struct CompletionCache;

impl CompletionCache {
    pub fn new() -> Self {
        Self
    }

    pub fn get(&self, _category: &str, _key: &str) -> Vec<String> {
        Vec::new()
    }

    pub fn get_best_matches(&self, _category: &str, _key: &str, _query: &str, _max_results: usize) -> Vec<String> {
        Vec::new()
    }

    pub fn update(&self, _category: &str, _key: &str, _values: Vec<String>) {
        // No-op implementation
    }

    pub fn has_category(&self, _category: &str) -> bool {
        false
    }

    pub fn has_key(&self, _category: &str, _key: &str) -> bool {
        false
    }
}

/// Path completer interface for file path completion
pub trait PathCompleter {
    /// Complete a file path
    fn complete(&self, path_prefix: &str) -> Vec<String>;
}

/// Adapter that provides read-only access to components needed for tab completion
///
/// This adapter extracts only the necessary components from ChatContext that command handlers need
/// for tab completion, providing immutable references to avoid accidental state modification.
pub struct CompletionContextAdapter<'a> {
    /// Conversation state access for reading history and messages
    pub conversation_state: &'a ConversationState,

    /// Tool permissions for checking trust status
    pub tool_permissions: &'a ToolPermissions,

    /// Completion cache for context-aware suggestions
    pub completion_cache: &'a CompletionCache,
    
    /// Path completer for file path completion
    pub path_completer: Option<&'a dyn PathCompleter>,
}

// Implement the PathCompleter trait for the prompt PathCompleter
impl PathCompleter for crate::cli::chat::prompt::PathCompleter {
    fn complete(&self, path_prefix: &str) -> Vec<String> {
        self.complete(path_prefix)
    }
}
