//! Paste Stack — ordered queue for sequential copy-then-paste workflows.

use std::sync::Mutex;
use crate::storage::models::Clip;

/// In-memory paste stack for sequential paste operations.
pub struct PasteStack {
    items: Mutex<Vec<Clip>>,
    active: Mutex<bool>,
}

impl PasteStack {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(Vec::new()),
            active: Mutex::new(false),
        }
    }

    /// Check if paste stack mode is active.
    pub fn is_active(&self) -> bool {
        *self.active.lock().unwrap()
    }

    /// Toggle paste stack mode. Returns the new active state.
    pub fn toggle(&self) -> bool {
        let mut active = self.active.lock().unwrap();
        *active = !*active;
        if !*active {
            // Clear stack when deactivating
            self.items.lock().unwrap().clear();
        }
        *active
    }

    /// Activate paste stack mode.
    pub fn activate(&self) {
        *self.active.lock().unwrap() = true;
    }

    /// Deactivate paste stack mode and clear the stack.
    pub fn deactivate(&self) {
        *self.active.lock().unwrap() = false;
        self.items.lock().unwrap().clear();
    }

    /// Add a clip to the end of the stack.
    pub fn push(&self, clip: Clip) {
        self.items.lock().unwrap().push(clip);
    }

    /// Pop the first item from the stack (FIFO).
    /// Returns None if the stack is empty.
    pub fn pop_next(&self) -> Option<Clip> {
        let mut items = self.items.lock().unwrap();
        if items.is_empty() {
            None
        } else {
            Some(items.remove(0))
        }
    }

    /// Get all items in the stack (for display).
    pub fn get_all(&self) -> Vec<Clip> {
        self.items.lock().unwrap().clone()
    }

    /// Remove an item from the stack by clip ID.
    pub fn remove(&self, clip_id: &str) -> bool {
        let mut items = self.items.lock().unwrap();
        let len_before = items.len();
        items.retain(|c| c.id != clip_id);
        items.len() < len_before
    }

    /// Move an item from one position to another.
    pub fn reorder(&self, from_index: usize, to_index: usize) -> bool {
        let mut items = self.items.lock().unwrap();
        if from_index >= items.len() || to_index >= items.len() {
            return false;
        }
        let item = items.remove(from_index);
        items.insert(to_index, item);
        true
    }

    /// Get the number of items in the stack.
    pub fn len(&self) -> usize {
        self.items.lock().unwrap().len()
    }

    /// Check if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.items.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_clip(id: &str, text: &str) -> Clip {
        Clip {
            id: id.to_string(),
            content_type: "text".to_string(),
            text_content: Some(text.to_string()),
            html_content: None,
            image_path: None,
            source_app: None,
            source_app_icon: None,
            content_hash: format!("hash_{}", id),
            content_size: text.len() as i64,
            metadata: None,
            pinboard_id: None,
            is_favorite: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            accessed_at: None,
            access_count: 0,
        }
    }

    #[test]
    fn test_new_stack_is_inactive_and_empty() {
        let stack = PasteStack::new();
        assert!(!stack.is_active());
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_toggle() {
        let stack = PasteStack::new();
        assert!(!stack.is_active());

        let active = stack.toggle();
        assert!(active);
        assert!(stack.is_active());

        let active = stack.toggle();
        assert!(!active);
        assert!(!stack.is_active());
    }

    #[test]
    fn test_toggle_clears_stack() {
        let stack = PasteStack::new();
        stack.activate();
        stack.push(make_clip("1", "hello"));
        stack.push(make_clip("2", "world"));
        assert_eq!(stack.len(), 2);

        stack.toggle(); // deactivate — should clear
        assert!(stack.is_empty());
    }

    #[test]
    fn test_push_and_pop_fifo() {
        let stack = PasteStack::new();
        stack.push(make_clip("1", "first"));
        stack.push(make_clip("2", "second"));
        stack.push(make_clip("3", "third"));

        let item = stack.pop_next().unwrap();
        assert_eq!(item.id, "1");

        let item = stack.pop_next().unwrap();
        assert_eq!(item.id, "2");

        let item = stack.pop_next().unwrap();
        assert_eq!(item.id, "3");

        assert!(stack.pop_next().is_none());
    }

    #[test]
    fn test_get_all() {
        let stack = PasteStack::new();
        stack.push(make_clip("1", "a"));
        stack.push(make_clip("2", "b"));

        let items = stack.get_all();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "1");
        assert_eq!(items[1].id, "2");
    }

    #[test]
    fn test_remove() {
        let stack = PasteStack::new();
        stack.push(make_clip("1", "a"));
        stack.push(make_clip("2", "b"));
        stack.push(make_clip("3", "c"));

        assert!(stack.remove("2"));
        let items = stack.get_all();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "1");
        assert_eq!(items[1].id, "3");
    }

    #[test]
    fn test_remove_nonexistent() {
        let stack = PasteStack::new();
        stack.push(make_clip("1", "a"));
        assert!(!stack.remove("999"));
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_reorder() {
        let stack = PasteStack::new();
        stack.push(make_clip("1", "a"));
        stack.push(make_clip("2", "b"));
        stack.push(make_clip("3", "c"));

        // Move item at index 2 to index 0
        assert!(stack.reorder(2, 0));
        let items = stack.get_all();
        assert_eq!(items[0].id, "3");
        assert_eq!(items[1].id, "1");
        assert_eq!(items[2].id, "2");
    }

    #[test]
    fn test_reorder_out_of_bounds() {
        let stack = PasteStack::new();
        stack.push(make_clip("1", "a"));
        assert!(!stack.reorder(0, 5));
        assert!(!stack.reorder(5, 0));
    }

    #[test]
    fn test_deactivate_clears() {
        let stack = PasteStack::new();
        stack.activate();
        stack.push(make_clip("1", "a"));
        assert_eq!(stack.len(), 1);

        stack.deactivate();
        assert!(!stack.is_active());
        assert!(stack.is_empty());
    }
}
