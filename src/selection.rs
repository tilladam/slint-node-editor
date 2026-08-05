use std::collections::HashSet;
use slint::{VecModel, Model};

/// Project a selection into a model's per-row `selected` flag.
///
/// The editor holds no selection state — rendering reads `selected` as model
/// data — so every write to the application's selection must be followed by a
/// projection into the rows. Only rows whose flag actually changes are written
/// back, so untouched rows don't re-render.
///
/// ```ignore
/// project_selection(
///     &nodes,
///     |n| selection.contains(n.id),   // wanted
///     |n| n.selected,                 // current
///     |n, v| n.selected = v);         // apply
/// ```
pub fn project_selection<T: Clone + 'static>(
    model: &VecModel<T>,
    wanted: impl Fn(&T) -> bool,
    current: impl Fn(&T) -> bool,
    mut apply: impl FnMut(&mut T, bool),
) {
    for i in 0..model.row_count() {
        let Some(row) = model.row_data(i) else { continue };
        let want = wanted(&row);
        if current(&row) != want {
            let mut row = row;
            apply(&mut row, want);
            model.set_row_data(i, row);
        }
    }
}

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

    /// Add a set of IDs to the current selection, leaving the rest in place
    ///
    /// Box selection with shift held; unlike [`Self::handle_interaction`] an
    /// already-selected ID stays selected instead of toggling off.
    pub fn extend_selection<I>(&mut self, ids: I)
    where
        I: IntoIterator<Item = i32>,
    {
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
    fn test_extend_selection_keeps_previous_and_reselects() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![1, 2]);

        selection.extend_selection(vec![2, 3]);

        assert_eq!(selection.len(), 3);
        assert!(selection.contains(1));
        // Already-selected IDs stay selected — extend never toggles
        assert!(selection.contains(2));
        assert!(selection.contains(3));
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
    // project_selection() - SSOT → per-row model data
    // ========================================================================

    #[derive(Clone, PartialEq, Debug)]
    struct Row {
        id: i32,
        selected: bool,
    }

    fn rows(ids: &[i32]) -> Rc<VecModel<Row>> {
        Rc::new(VecModel::from(
            ids.iter()
                .map(|&id| Row {
                    id,
                    selected: false,
                })
                .collect::<Vec<_>>(),
        ))
    }

    fn project(model: &VecModel<Row>, selection: &SelectionManager) {
        project_selection(
            model,
            |r| selection.contains(r.id),
            |r| r.selected,
            |r, v| r.selected = v,
        );
    }

    #[test]
    fn test_project_selection_marks_selected_rows() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![2]);

        let model = rows(&[1, 2, 3]);
        project(&model, &selection);

        let flags: Vec<bool> = (0..model.row_count())
            .filter_map(|i| model.row_data(i))
            .map(|r| r.selected)
            .collect();
        assert_eq!(flags, vec![false, true, false]);
    }

    #[test]
    fn test_project_selection_clears_stale_flags() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![1]);

        let model = rows(&[1, 2]);
        project(&model, &selection);

        selection.replace_selection(vec![2]);
        project(&model, &selection);

        assert!(!model.row_data(0).unwrap().selected);
        assert!(model.row_data(1).unwrap().selected);
    }

    #[test]
    fn test_project_selection_only_writes_changed_rows() {
        let mut selection = SelectionManager::new();
        selection.replace_selection(vec![1]);

        let model = rows(&[1, 2]);
        project(&model, &selection);

        // A second projection of the same selection must be a no-op — track it
        // by counting rows the projection would rewrite.
        let mut rewritten = 0;
        project_selection(
            &model,
            |r| selection.contains(r.id),
            |r| r.selected,
            |r, v| {
                rewritten += 1;
                r.selected = v;
            },
        );
        assert_eq!(rewritten, 0);
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