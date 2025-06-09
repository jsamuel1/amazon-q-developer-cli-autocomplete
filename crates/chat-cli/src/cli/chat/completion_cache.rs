use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, RwLock};

/// A cache for command completion suggestions organized by category and key
///
/// This cache provides a hierarchical structure for storing and retrieving completion suggestions:
/// - Categories represent broad groups like "profiles", "context_files", "tools", etc.
/// - Keys represent subgroups within categories like "all", "current", "trusted", etc.
/// - Values are stored in BTreeSets for automatic sorting and uniqueness
///
/// The cache also provides fuzzy matching capabilities.
pub struct CompletionCache {
    /// Main cache structure: category -> key -> sorted values
    cache: Arc<RwLock<HashMap<String, HashMap<String, BTreeSet<String>>>>>,
}

impl CompletionCache {
    /// Create a new empty completion cache
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get all values for a category and key as a sorted Vec
    ///
    /// # Arguments
    ///
    /// * `category` - The category to get values from (e.g., "profiles", "tools")
    /// * `key` - The key within the category (e.g., "all", "trusted")
    ///
    /// # Returns
    ///
    /// A vector of strings containing all values for the given category and key,
    /// or an empty vector if the category or key doesn't exist.
    pub fn get(&self, category: &str, key: &str) -> Vec<String> {
        self.cache
            .read()
            .unwrap()
            .get(category)
            .and_then(|map| map.get(key))
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Update values for a category and key
    ///
    /// # Arguments
    ///
    /// * `category` - The category to update (e.g., "profiles", "tools")
    /// * `key` - The key within the category (e.g., "all", "trusted")
    /// * `values` - The new values to store
    pub fn update(&self, category: &str, key: &str, values: Vec<String>) {
        let mut cache = self.cache.write().unwrap();
        let category_map = cache.entry(category.to_string()).or_insert_with(HashMap::new);
        let set = category_map.entry(key.to_string()).or_insert_with(BTreeSet::new);
        set.clear();
        set.extend(values);
    }
    
    /// Add a single value to a category and key
    ///
    /// # Arguments
    ///
    /// * `category` - The category to update (e.g., "profiles", "tools")
    /// * `key` - The key within the category (e.g., "all", "trusted")
    /// * `value` - The value to add
    pub fn add(&self, category: &str, key: &str, value: String) {
        let mut cache = self.cache.write().unwrap();
        let category_map = cache.entry(category.to_string()).or_insert_with(HashMap::new);
        let set = category_map.entry(key.to_string()).or_insert_with(BTreeSet::new);
        set.insert(value);
    }
    
    /// Remove a single value from a category and key
    ///
    /// # Arguments
    ///
    /// * `category` - The category to update (e.g., "profiles", "tools")
    /// * `key` - The key within the category (e.g., "all", "trusted")
    /// * `value` - The value to remove
    pub fn remove(&self, category: &str, key: &str, value: &str) {
        if let Some(cache) = self.cache.write().unwrap().get_mut(category) {
            if let Some(set) = cache.get_mut(key) {
                set.remove(value);
            }
        }
    }
    
    /// Clear all values for a category and key
    ///
    /// # Arguments
    ///
    /// * `category` - The category to clear (e.g., "profiles", "tools")
    /// * `key` - The key within the category (e.g., "all", "trusted")
    pub fn clear(&self, category: &str, key: &str) {
        if let Some(cache) = self.cache.write().unwrap().get_mut(category) {
            if let Some(set) = cache.get_mut(key) {
                set.clear();
            }
        }
    }
    
    /// Get fuzzy-matched values for a category and key
    ///
    /// # Arguments
    ///
    /// * `category` - The category to get values from (e.g., "profiles", "tools")
    /// * `key` - The key within the category (e.g., "all", "trusted")
    /// * `query` - The query string to match against
    ///
    /// # Returns
    ///
    /// A vector of (string, score) pairs containing all values that match the query,
    /// sorted by score (highest first).
    pub fn get_fuzzy_matches(&self, category: &str, key: &str, query: &str) -> Vec<(String, i64)> {
        let cache_read = self.cache.read().unwrap();
        
        if let Some(category_map) = cache_read.get(category) {
            if let Some(items) = category_map.get(key) {
                // Simple prefix matching as fallback
                let mut matches: Vec<(String, i64)> = items
                    .iter()
                    .filter_map(|item| {
                        if item.starts_with(query) {
                            // Higher score for exact prefix matches
                            Some((item.clone(), 100))
                        } else if item.to_lowercase().contains(&query.to_lowercase()) {
                            // Lower score for substring matches
                            Some((item.clone(), 50))
                        } else {
                            None
                        }
                    })
                    .collect();
                
                // Sort by score (highest first)
                matches.sort_by(|a, b| b.1.cmp(&a.1));
                return matches;
            }
        }
        
        Vec::new()
    }
    
    /// Get the best matches for a query (limited to max_results)
    ///
    /// # Arguments
    ///
    /// * `category` - The category to get values from (e.g., "profiles", "tools")
    /// * `key` - The key within the category (e.g., "all", "trusted")
    /// * `query` - The query string to match against
    /// * `max_results` - The maximum number of results to return
    ///
    /// # Returns
    ///
    /// A vector of strings containing the best matches for the query,
    /// limited to max_results.
    pub fn get_best_matches(&self, category: &str, key: &str, query: &str, max_results: usize) -> Vec<String> {
        self.get_fuzzy_matches(category, key, query)
            .into_iter()
            .take(max_results)
            .map(|(item, _)| item)
            .collect()
    }
    
    /// Check if a category exists
    ///
    /// # Arguments
    ///
    /// * `category` - The category to check
    ///
    /// # Returns
    ///
    /// `true` if the category exists, `false` otherwise
    pub fn has_category(&self, category: &str) -> bool {
        self.cache.read().unwrap().contains_key(category)
    }
    
    /// Check if a key exists within a category
    ///
    /// # Arguments
    ///
    /// * `category` - The category to check
    /// * `key` - The key to check
    ///
    /// # Returns
    ///
    /// `true` if the key exists within the category, `false` otherwise
    pub fn has_key(&self, category: &str, key: &str) -> bool {
        self.cache
            .read()
            .unwrap()
            .get(category)
            .map(|map| map.contains_key(key))
            .unwrap_or(false)
    }
    
    /// Get all categories
    ///
    /// # Returns
    ///
    /// A vector of strings containing all category names
    pub fn get_categories(&self) -> Vec<String> {
        self.cache
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }
    
    /// Get all keys for a category
    ///
    /// # Arguments
    ///
    /// * `category` - The category to get keys from
    ///
    /// # Returns
    ///
    /// A vector of strings containing all keys for the given category,
    /// or an empty vector if the category doesn't exist.
    pub fn get_keys(&self, category: &str) -> Vec<String> {
        self.cache
            .read()
            .unwrap()
            .get(category)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_update_and_get() {
        let cache = CompletionCache::new();
        let values = vec!["value1".to_string(), "value2".to_string()];
        
        cache.update("category", "key", values.clone());
        
        let result = cache.get("category", "key");
        assert_eq!(result, values);
    }
    
    #[test]
    fn test_add_and_remove() {
        let cache = CompletionCache::new();
        
        cache.add("category", "key", "value1".to_string());
        cache.add("category", "key", "value2".to_string());
        
        let result = cache.get("category", "key");
        assert_eq!(result, vec!["value1".to_string(), "value2".to_string()]);
        
        cache.remove("category", "key", "value1");
        
        let result = cache.get("category", "key");
        assert_eq!(result, vec!["value2".to_string()]);
    }
    
    #[test]
    fn test_clear() {
        let cache = CompletionCache::new();
        
        cache.add("category", "key", "value1".to_string());
        cache.add("category", "key", "value2".to_string());
        
        cache.clear("category", "key");
        
        let result = cache.get("category", "key");
        assert!(result.is_empty());
    }
    
    #[test]
    fn test_fuzzy_matching() {
        let cache = CompletionCache::new();
        
        cache.update("category", "key", vec![
            "apple".to_string(),
            "banana".to_string(),
            "cherry".to_string(),
            "date".to_string(),
        ]);
        
        // Test exact match
        let matches = cache.get_best_matches("category", "key", "apple", 10);
        assert_eq!(matches, vec!["apple".to_string()]);
        
        // Test prefix match
        let matches = cache.get_best_matches("category", "key", "ap", 10);
        assert_eq!(matches, vec!["apple".to_string()]);
        
        // Test multiple matches
        let matches = cache.get_best_matches("category", "key", "a", 10);
        assert!(matches.contains(&"apple".to_string()));
        assert!(matches.contains(&"banana".to_string()));
        assert!(matches.contains(&"date".to_string()));
        
        // Test limit
        let matches = cache.get_best_matches("category", "key", "a", 1);
        assert_eq!(matches.len(), 1);
    }
    
    #[test]
    fn test_has_category_and_key() {
        let cache = CompletionCache::new();
        
        cache.add("category", "key", "value".to_string());
        
        assert!(cache.has_category("category"));
        assert!(cache.has_key("category", "key"));
        assert!(!cache.has_category("nonexistent"));
        assert!(!cache.has_key("category", "nonexistent"));
    }
    
    #[test]
    fn test_get_categories_and_keys() {
        let cache = CompletionCache::new();
        
        cache.add("category1", "key1", "value".to_string());
        cache.add("category1", "key2", "value".to_string());
        cache.add("category2", "key3", "value".to_string());
        
        let categories = cache.get_categories();
        assert_eq!(categories.len(), 2);
        assert!(categories.contains(&"category1".to_string()));
        assert!(categories.contains(&"category2".to_string()));
        
        let keys = cache.get_keys("category1");
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }
}
