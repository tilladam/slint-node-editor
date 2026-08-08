//! Host-side selection helpers.
//!
//! The editor holds no selection state. It reports gestures — a node or link
//! was clicked, the background was clicked, a marquee was released — and
//! renders the `selected` flag it finds on the rows of your models. Everything
//! between those two ends is the host's, and this module is what the host
//! needs to do it:
//!
//! - [`resolve_click`] and [`resolve_box`] hold the *policy*: what a click,
//!   a shift-click, a marquee and a shift-marquee mean. They are pure — no
//!   storage, no interior mutability — so the host is free to keep the
//!   selection wherever it already keeps its state.
//! - [`project_selection`] writes a selection back into the rows, and
//!   [`selected_rows`] reads it back out, so a host that wants no separate
//!   store can let the rows *be* the selection.
//!
//! # Contract
//!
//! Every intent resolves to an **absolute set**: the resolvers take the
//! current selection and return the new one in full, never a delta. There is
//! no "add" or "remove" call, and no reachable state in which a selection was
//! half-applied.
//!
//! # Order
//!
//! Both resolvers are **order-stable**: surviving members keep their relative
//! order and new members are appended. Order is presentation-stable and
//! semantically meaningless — it is what an introspecting client sees, and
//! nothing may interpret it. If you ever need a distinguished member (a paste
//! anchor, a "primary" selection), that is an explicit concept to add, never a
//! position in this vector.

use slint::Model;

/// Resolve a click on `item` against the current selection.
///
/// Without shift a click replaces the selection with `item`. With shift it
/// toggles: an unselected item joins the end, a selected one drops out and
/// the rest keep their order.
pub fn resolve_click<T: Clone + PartialEq>(current: &[T], item: T, shift: bool) -> Vec<T> {
    if !shift {
        return vec![item];
    }
    if current.contains(&item) {
        current.iter().filter(|x| **x != item).cloned().collect()
    } else {
        let mut extended = Vec::with_capacity(current.len() + 1);
        extended.extend_from_slice(current);
        extended.push(item);
        extended
    }
}

/// Resolve a released marquee against the current selection.
///
/// Without shift the hits become the selection. With shift the marquee
/// extends it: the current selection keeps its order, and whichever hits
/// aren't already in it are appended. Extending never toggles — a hit that is
/// already selected stays selected.
pub fn resolve_box<T: Clone + PartialEq>(current: &[T], hits: Vec<T>, shift: bool) -> Vec<T> {
    if !shift {
        return hits;
    }
    let mut extended = current.to_vec();
    for hit in hits {
        if !extended.contains(&hit) {
            extended.push(hit);
        }
    }
    extended
}

/// Project a selection into a model's per-row `selected` flag.
///
/// Rendering reads `selected` as model data, so every write to the host's
/// selection must be followed by a projection into the rows. `wanted` decides
/// whether a row should be selected; `flag` hands back the row's own flag,
/// which is both read and written — one accessor, so the read and the write
/// can never disagree about which field holds the answer.
///
/// Only rows whose flag actually changes are written back, so untouched rows
/// don't re-render.
///
/// ```ignore
/// project_selection(&nodes, |n| selection.contains(&n.id), |n| &mut n.selected);
/// ```
pub fn project_selection<T: Clone + 'static>(
    model: &impl Model<Data = T>,
    wanted: impl Fn(&T) -> bool,
    flag: impl Fn(&mut T) -> &mut bool,
) {
    for i in 0..model.row_count() {
        let Some(mut row) = model.row_data(i) else {
            continue;
        };
        let want = wanted(&row);
        let current = flag(&mut row);
        if *current != want {
            *current = want;
            model.set_row_data(i, row);
        }
    }
}

/// Apply a click to a model whose rows carry the selection.
///
/// Reads the current selection out of the rows, resolves the click against it,
/// and projects the answer back — the whole gesture, with no state in between.
/// `flag` is the row's own selected field, used for both halves.
pub fn apply_click<T: Clone + 'static, K: Clone + PartialEq>(
    model: &impl Model<Data = T>,
    key: impl Fn(&T) -> K,
    flag: impl Fn(&mut T) -> &mut bool,
    item: K,
    shift: bool,
) {
    let current = collect_selected(model, &key, &flag);
    let next = resolve_click(&current, item, shift);
    project_selection(model, |row| next.contains(&key(row)), &flag);
}

/// Apply a released marquee to a model whose rows carry the selection.
///
/// The mirror of [`apply_click`] for [`resolve_box`].
pub fn apply_box<T: Clone + 'static, K: Clone + PartialEq>(
    model: &impl Model<Data = T>,
    key: impl Fn(&T) -> K,
    flag: impl Fn(&mut T) -> &mut bool,
    hits: Vec<K>,
    shift: bool,
) {
    let current = collect_selected(model, &key, &flag);
    let next = resolve_box(&current, hits, shift);
    project_selection(model, |row| next.contains(&key(row)), &flag);
}

/// Deselect every row of a model.
pub fn clear_selection<T: Clone + 'static>(
    model: &impl Model<Data = T>,
    flag: impl Fn(&mut T) -> &mut bool,
) {
    project_selection(model, |_| false, flag);
}

/// Read the selection through the same mutable accessor the writers use, so a
/// gesture never needs a second accessor that could disagree with it.
fn collect_selected<T: Clone + 'static, K>(
    model: &impl Model<Data = T>,
    key: &impl Fn(&T) -> K,
    flag: &impl Fn(&mut T) -> &mut bool,
) -> Vec<K> {
    (0..model.row_count())
        .filter_map(|i| model.row_data(i))
        .filter_map(|mut row| {
            if *flag(&mut row) {
                Some(key(&row))
            } else {
                None
            }
        })
        .collect()
}

/// Collect a key for every row the model currently shows as selected.
///
/// The rows are a complete record of the selection, so this is how a host
/// with no separate selection store reads the current set before resolving a
/// gesture against it. Results come back in row order — the rows do not
/// remember the order the selection was built in.
pub fn selected_rows<T: Clone + 'static, K>(
    model: &impl Model<Data = T>,
    key: impl Fn(&T) -> K,
    flag: impl Fn(&T) -> bool,
) -> Vec<K> {
    (0..model.row_count())
        .filter_map(|i| model.row_data(i))
        .filter(|row| flag(row))
        .map(|row| key(&row))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slint::VecModel;
    use std::rc::Rc;

    // ========================================================================
    // resolve_click() — click policy
    // ========================================================================

    #[test]
    fn click_replaces_the_selection() {
        assert_eq!(resolve_click(&[1, 2, 3], 9, false), vec![9]);
    }

    #[test]
    fn click_on_the_only_selected_item_is_idempotent() {
        assert_eq!(resolve_click(&[7], 7, false), vec![7]);
    }

    #[test]
    fn shift_click_appends_an_unselected_item() {
        assert_eq!(resolve_click(&[1, 2], 3, true), vec![1, 2, 3]);
    }

    #[test]
    fn shift_click_removes_a_selected_item_and_keeps_order() {
        assert_eq!(resolve_click(&[1, 2, 3], 2, true), vec![1, 3]);
    }

    #[test]
    fn shift_click_on_an_empty_selection_selects() {
        assert_eq!(resolve_click(&[], 4, true), vec![4]);
    }

    #[test]
    fn click_policy_is_not_id_specific() {
        // The resolvers are generic so hosts can key selection by whatever
        // identity they already have — here a link's (output, input) pair.
        let current = [(1, 2), (3, 4)];
        assert_eq!(resolve_click(&current, (3, 4), true), vec![(1, 2)]);
    }

    // ========================================================================
    // resolve_box() — marquee policy
    // ========================================================================

    #[test]
    fn box_replaces_the_selection() {
        assert_eq!(resolve_box(&[1, 2], vec![5, 6], false), vec![5, 6]);
    }

    #[test]
    fn empty_box_clears_without_shift() {
        assert_eq!(resolve_box(&[1, 2], Vec::new(), false), Vec::<i32>::new());
    }

    #[test]
    fn shift_box_keeps_current_order_and_appends_new_hits() {
        assert_eq!(resolve_box(&[3, 1], vec![4, 5], true), vec![3, 1, 4, 5]);
    }

    #[test]
    fn shift_box_never_toggles_an_already_selected_hit() {
        // Unlike shift-click, a hit that is already selected stays selected.
        assert_eq!(resolve_box(&[1, 2], vec![2, 3], true), vec![1, 2, 3]);
    }

    #[test]
    fn shift_box_with_no_hits_is_a_no_op() {
        assert_eq!(resolve_box(&[1, 2], Vec::new(), true), vec![1, 2]);
    }

    // ========================================================================
    // project_selection() — selection → per-row model data
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

    fn project(model: &VecModel<Row>, selection: &[i32]) {
        project_selection(model, |r| selection.contains(&r.id), |r| &mut r.selected);
    }

    fn flags(model: &VecModel<Row>) -> Vec<bool> {
        (0..model.row_count())
            .filter_map(|i| model.row_data(i))
            .map(|r| r.selected)
            .collect()
    }

    #[test]
    fn project_marks_selected_rows() {
        let model = rows(&[1, 2, 3]);
        project(&model, &[2]);
        assert_eq!(flags(&model), vec![false, true, false]);
    }

    #[test]
    fn project_clears_stale_flags() {
        let model = rows(&[1, 2]);
        project(&model, &[1]);
        project(&model, &[2]);
        assert_eq!(flags(&model), vec![false, true]);
    }

    #[test]
    fn project_only_writes_changed_rows() {
        let model = rows(&[1, 2]);
        project(&model, &[1]);

        // A second projection of the same selection must write nothing. The
        // model itself is the witness: a rewrite would have to change a flag,
        // and none of them move.
        let before = flags(&model);
        project(&model, &[1]);
        assert_eq!(flags(&model), before);
        assert_eq!(before, vec![true, false]);
    }

    #[test]
    fn project_into_an_empty_model_is_a_no_op() {
        let model = rows(&[]);
        project(&model, &[1, 2]);
        assert_eq!(flags(&model), Vec::<bool>::new());
    }

    // ========================================================================
    // selected_rows() — the rows are the selection
    // ========================================================================

    #[test]
    fn selected_rows_reads_back_what_was_projected() {
        let model = rows(&[1, 2, 3]);
        project(&model, &[3, 1]);

        // Row order, not selection order — the rows don't remember the latter.
        assert_eq!(selected_rows(&model, |r| r.id, |r| r.selected), vec![1, 3]);
    }

    #[test]
    fn selected_rows_is_empty_when_nothing_is_selected() {
        let model = rows(&[1, 2]);
        assert_eq!(
            selected_rows(&model, |r| r.id, |r| r.selected),
            Vec::<i32>::new()
        );
    }

    // ========================================================================
    // Round trip: the rows as the only selection store
    // ========================================================================

    #[test]
    fn gesture_round_trip_through_the_rows() {
        let model = rows(&[1, 2, 3]);

        // Click 2, then shift-click 3, then shift-click 2 back off.
        for (item, shift) in [(2, false), (3, true), (2, true)] {
            let current = selected_rows(&model, |r| r.id, |r| r.selected);
            let next = resolve_click(&current, item, shift);
            project(&model, &next);
        }

        assert_eq!(flags(&model), vec![false, false, true]);
    }

    #[test]
    fn negative_ids_work() {
        assert_eq!(resolve_click(&[-1], -2, true), vec![-1, -2]);
    }
}
