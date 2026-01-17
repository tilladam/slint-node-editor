use std::collections::HashSet;
use slint::{VecModel, Model};

#[derive(Default)]
pub struct SelectionManager {
    selected: HashSet<i32>,
}

impl SelectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle selection of an item (e.g., node or link) based on interaction modifiers
    pub fn handle_interaction(&mut self, id: i32, shift_held: bool) {
        if shift_held {
            if self.selected.contains(&id) {
                self.selected.remove(&id);
            } else {
                self.selected.insert(id);
            }
        } else {
            if self.selected.len() == 1 && self.selected.contains(&id) {
                return;
            }
            self.selected.clear();
            self.selected.insert(id);
        }
    }

    /// Clear the current selection
    pub fn clear(&mut self) {
        self.selected.clear();
    }

    /// Replace the current selection with a new set of IDs
    /// 
    /// Useful for box selection sync
    pub fn replace_selection<I>(&mut self, ids: I)
    where
        I: IntoIterator<Item = i32>,
    {
        self.selected.clear();
        self.selected.extend(ids);
    }

    /// Check if an ID is selected
    pub fn contains(&self, id: i32) -> bool {
        self.selected.contains(&id)
    }

    /// Get an iterator over the selected IDs
    pub fn iter(&self) -> std::collections::hash_set::Iter<'_, i32> {
        self.selected.iter()
    }

    /// Sync the internal selection set to a Slint VecModel
    pub fn sync_to_model(&self, model: &VecModel<i32>) {
        // Clear and repopulate to ensure exact match
        while model.row_count() > 0 {
            model.remove(0);
        }
        for &id in &self.selected {
            model.push(id);
        }
    }

    /// Sync the internal selection set from any Slint Model (e.g. after box selection)
    pub fn sync_from_model(&mut self, model: &dyn Model<Data = i32>) {
        self.selected.clear();
        for i in 0..model.row_count() {
            if let Some(id) = model.row_data(i) {
                self.selected.insert(id);
            }
        }
    }

    /// Get the number of selected items
    pub fn len(&self) -> usize {
        self.selected.len()
    }

    /// Check if the selection is empty
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Rc;

    // ========================================================================
    // SelectionManager::new() and Default
    // ========================================================================

    #[test]
    fn test_new_selection_is_empty() {
        let selection = SelectionManager::new();
        assert!(selection.is_empty());
        assert_eq!(selection.len(), 0);
    }

    #[test]
    fn test_default_selection_is_empty() {
        let selection = SelectionManager::default();
        assert!(selection.is_empty());
    }

    // ========================================================================
    // contains() - Basic HashSet operations
    // ========================================================================

    #[test]
    fn test_contains_returns_false_for_empty() {
        let selection = SelectionManager::new();
        assert!(!selection.contains(1));
        assert!(!selection.contains(0));
        assert!(!selection.contains(-1));
    }

    #[test]
    fn test_contains_returns_true_for_selected() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(42, false);
        assert!(selection.contains(42));
    }

    // ========================================================================
    // handle_interaction() - State Machine Behavior
    // ========================================================================

    #[test]
    fn test_handle_interaction_click_selects_single() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, false);

        assert!(selection.contains(1));
        assert_eq!(selection.len(), 1);
    }

    #[test]
    fn test_handle_interaction_click_replaces_selection() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, false);
        selection.handle_interaction(2, false);

        assert!(!selection.contains(1));
        assert!(selection.contains(2));
        assert_eq!(selection.len(), 1);
    }

    #[test]
    fn test_handle_interaction_click_on_already_selected_single_is_noop() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, false);
        selection.handle_interaction(1, false); // Click again

        // Should still be selected (single item case)
        assert!(selection.contains(1));
        assert_eq!(selection.len(), 1);
    }

    #[test]
    fn test_handle_interaction_click_on_already_selected_in_multi_collapses() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, true); // Shift+click
        selection.handle_interaction(2, true); // Shift+click

        assert_eq!(selection.len(), 2);

        // Normal click on one - should collapse to just that one
        selection.handle_interaction(1, false);

        assert!(selection.contains(1));
        assert!(!selection.contains(2));
        assert_eq!(selection.len(), 1);
    }

    #[test]
    fn test_handle_interaction_shift_click_adds_to_selection() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, false);
        selection.handle_interaction(2, true); // Shift+click

        assert!(selection.contains(1));
        assert!(selection.contains(2));
        assert_eq!(selection.len(), 2);
    }

    #[test]
    fn test_handle_interaction_shift_click_toggles_off() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, false);
        selection.handle_interaction(2, true);

        assert_eq!(selection.len(), 2);

        // Shift+click on already selected removes it
        selection.handle_interaction(1, true);

        assert!(!selection.contains(1));
        assert!(selection.contains(2));
        assert_eq!(selection.len(), 1);
    }

    #[test]
    fn test_handle_interaction_shift_click_on_empty_adds() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, true); // Shift+click on empty

        assert!(selection.contains(1));
        assert_eq!(selection.len(), 1);
    }

    #[test]
    fn test_handle_interaction_shift_click_toggle_all_off() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, true);
        selection.handle_interaction(1, true); // Toggle off

        assert!(!selection.contains(1));
        assert!(selection.is_empty());
    }

    // ========================================================================
    // clear() - Selection Clearing
    // ========================================================================

    #[test]
    fn test_clear_empties_selection() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, false);
        selection.handle_interaction(2, true);

        selection.clear();

        assert!(selection.is_empty());
        assert!(!selection.contains(1));
        assert!(!selection.contains(2));
    }

    #[test]
    fn test_clear_on_empty_is_noop() {
        let mut selection = SelectionManager::new();
        selection.clear();
        assert!(selection.is_empty());
    }

    // ========================================================================
    // replace_selection() - Box Selection Sync
    // ========================================================================

    #[test]
    fn test_replace_selection_sets_new_items() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![1, 2, 3]);

        assert!(selection.contains(1));
        assert!(selection.contains(2));
        assert!(selection.contains(3));
        assert_eq!(selection.len(), 3);
    }

    #[test]
    fn test_replace_selection_clears_previous() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(10, false);

        selection.replace_selection(vec![1, 2]);

        assert!(!selection.contains(10));
        assert!(selection.contains(1));
        assert!(selection.contains(2));
    }

    #[test]
    fn test_replace_selection_with_empty_clears_all() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, false);

        selection.replace_selection(Vec::<i32>::new());

        assert!(selection.is_empty());
    }

    #[test]
    fn test_replace_selection_deduplicates() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![1, 2, 1, 2, 1]); // Duplicates

        assert_eq!(selection.len(), 2); // HashSet deduplicates
    }

    #[test]
    fn test_replace_selection_idempotent() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![1, 2, 3]);

        let count_before = selection.len();

        selection.replace_selection(vec![1, 2, 3]); // Same items

        assert_eq!(selection.len(), count_before);
    }

    // ========================================================================
    // iter() - Iteration
    // ========================================================================

    #[test]
    fn test_iter_returns_all_selected() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![1, 2, 3]);

        let mut items: Vec<i32> = selection.iter().copied().collect();
        items.sort();

        assert_eq!(items, vec![1, 2, 3]);
    }

    #[test]
    fn test_iter_empty_selection() {
        let selection = SelectionManager::new();
        assert_eq!(selection.iter().count(), 0);
    }

    // ========================================================================
    // sync_to_model() - Export to VecModel
    // ========================================================================

    #[test]
    fn test_sync_to_model_populates_empty_model() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![1, 2, 3]);

        let model: Rc<VecModel<i32>> = Rc::new(VecModel::default());
        selection.sync_to_model(&model);

        assert_eq!(model.row_count(), 3);
    }

    #[test]
    fn test_sync_to_model_clears_existing_data() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![1]);

        let model: Rc<VecModel<i32>> = Rc::new(VecModel::from(vec![10, 20, 30]));
        selection.sync_to_model(&model);

        assert_eq!(model.row_count(), 1);
        // The old values should be gone
        let values: Vec<i32> = (0..model.row_count())
            .filter_map(|i| model.row_data(i))
            .collect();
        assert!(values.contains(&1));
        assert!(!values.contains(&10));
    }

    #[test]
    fn test_sync_to_model_empty_selection() {
        let selection = SelectionManager::new();

        let model: Rc<VecModel<i32>> = Rc::new(VecModel::from(vec![1, 2, 3]));
        selection.sync_to_model(&model);

        assert_eq!(model.row_count(), 0);
    }

    // ========================================================================
    // sync_from_model() - Import from Model
    // ========================================================================

    #[test]
    fn test_sync_from_model_imports_items() {
        let mut selection = SelectionManager::new();
        let model: Rc<VecModel<i32>> = Rc::new(VecModel::from(vec![1, 2, 3]));

        selection.sync_from_model(model.as_ref());

        assert!(selection.contains(1));
        assert!(selection.contains(2));
        assert!(selection.contains(3));
        assert_eq!(selection.len(), 3);
    }

    #[test]
    fn test_sync_from_model_clears_existing() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(10, false);

        let model: Rc<VecModel<i32>> = Rc::new(VecModel::from(vec![1, 2]));
        selection.sync_from_model(model.as_ref());

        assert!(!selection.contains(10));
        assert!(selection.contains(1));
        assert!(selection.contains(2));
    }

    #[test]
    fn test_sync_from_model_empty_model() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(1, false);

        let model: Rc<VecModel<i32>> = Rc::new(VecModel::default());
        selection.sync_from_model(model.as_ref());

        assert!(selection.is_empty());
    }

    // ========================================================================
    // Round-trip: sync_to_model then sync_from_model
    // ========================================================================

    #[test]
    fn test_sync_roundtrip_preserves_selection() {
        let mut selection1 = SelectionManager::new();
        selection1.replace_selection(vec![1, 2, 3]);

        let model: Rc<VecModel<i32>> = Rc::new(VecModel::default());
        selection1.sync_to_model(&model);

        let mut selection2 = SelectionManager::new();
        selection2.sync_from_model(model.as_ref());

        // Both should have the same items
        assert!(selection2.contains(1));
        assert!(selection2.contains(2));
        assert!(selection2.contains(3));
        assert_eq!(selection2.len(), 3);
    }

    // ========================================================================
    // Edge Cases
    // ========================================================================

    #[test]
    fn test_negative_ids_work() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(-1, false);
        selection.handle_interaction(-2, true);

        assert!(selection.contains(-1));
        assert!(selection.contains(-2));
    }

    #[test]
    fn test_zero_id_works() {
        let mut selection = SelectionManager::new();
        selection.handle_interaction(0, false);

        assert!(selection.contains(0));
    }

    #[test]
    fn test_large_selection() {
        let mut selection = SelectionManager::new();
        let ids: Vec<i32> = (0..1000).collect();
        selection.replace_selection(ids);

        assert_eq!(selection.len(), 1000);
        assert!(selection.contains(0));
        assert!(selection.contains(500));
        assert!(selection.contains(999));
    }
}