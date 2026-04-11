# Accessibility Plan for slint-node-editor

## Context

Node editors are notoriously inaccessible. The library currently has basic
accessible roles (list/list-item/button) but no keyboard navigation, no
link accessibility, and no dynamic announcements. This plan addresses
accessibility systematically, informed by WAI-ARIA best practices, React
Flow's implementation (the best-in-class web reference), and Slint's
accessibility facilities.

The goal: a blind or keyboard-only user should be able to navigate nodes,
understand connections, select and delete elements, and create links —
all without a mouse.

## Current state

| Component | Role | Keyboard | Screen reader |
|-----------|------|----------|---------------|
| NodeEditor | list | FocusScope (no key bindings) | "Node Editor" |
| BaseNode | list-item | Not focusable | "Node {id}", "selected" |
| Pin | button | Not focusable | "input/output pin" |
| Link | (none) | N/A | Invisible |
| Minimap | (none) | N/A | Invisible |

**Gaps:** No keyboard navigation to nodes/pins. Links invisible to assistive
tech. No announcements for state changes. No keyboard link creation.

## Slint facilities available

**Available (23 properties):**
- `accessible-role` (19 roles including list, list-item, button)
- `accessible-label`, `accessible-description`, `accessible-value`
- `accessible-item-selected`, `accessible-item-selectable`
- `accessible-item-index`, `accessible-item-count`
- `accessible-checked`, `accessible-checkable`
- `accessible-expanded`, `accessible-expandable`
- `accessible-action-default`, `accessible-action-increment/decrement`
- `accessible-action-set-value(String)`, `accessible-action-expand`
- `accessible-delegate-focus` (parent delegates focus to child by index)
- `accessible-enabled`, `accessible-read-only`

**NOT available (would need Slint extensions):**
- Live regions (aria-live) — no way to announce dynamic changes
- Accessible relationships (aria-flowto, aria-controls) — no way to
  express "node A connects to node B"
- Custom focus order — only delegate-focus by child index

## Tiered implementation plan

### Tier 1: Structural keyboard navigation (library)

**Goal:** Nodes are focusable, navigable via keyboard, with visible focus indicators.

**1a. Make BaseNode focusable**

BaseNode needs to participate in the focus chain. Two approaches:
- BaseNode inherits FocusScope (changes inheritance from Rectangle)
- Add a FocusScope child inside BaseNode

Recommendation: Add a FocusScope child (preserves Rectangle inheritance, avoids
breaking existing code). The FocusScope covers the full node area and handles
key-pressed for node-level navigation.

**1b. Add focused-node tracking to NodeEditor**

NodeEditor needs:
```slint
// Internal state
property <int> focused-node-index: -1;  // -1 = no node focused
property <int> node-count: 0;           // set by application

// Delegate focus to the focused node
accessible-delegate-focus: focused-node-index;
```

Tab/Shift+Tab in NodeEditor's key-pressed handler cycles `focused-node-index`
through `0..node-count-1`. This is the same pattern Slint's TabWidget uses.

**1c. Visual focus indicator on BaseNode**

When a node has focus, render a visible focus ring:
```slint
// In BaseNode, a Rectangle child that shows when focused
Rectangle {
    visible: root.has-focus;  // or a focused property
    border-width: 2px;
    border-color: #4a9eff;
    border-radius: root.border-radius-base + 2px;
    // Slightly larger than node to create ring effect
}
```

**1d. Accessible item properties on BaseNode**

```slint
accessible-item-selectable: true;
accessible-item-selected: root.selected;
accessible-item-index: ???;  // needs to be passed from repeater
```

The `item-index` is tricky — the repeater index isn't automatically available
to the child component. Options:
- Add an `in property <int> node-index` to BaseNode, set by the repeater
- Use `node-id` as the index (but it's not sequential)
- Compute from the model

Recommendation: Add `in property <int> node-index: -1` to BaseNode. Examples
set it from the repeater: `node-index: index` (Slint repeaters expose `index`).

**Files:** `node-editor-building-blocks.slint` (BaseNode), `node-editor.slint` (NodeEditor)

---

### Tier 2: Node keyboard operations (library provides, embedder binds)

**Goal:** Focused nodes can be selected, moved, and deleted via keyboard.

The library already separates focus (structural) from keymaps (policy). For
accessibility, the library should provide *functions* that embedders bind:

**2a. New public functions on NodeEditor:**

```slint
/// Move focus to the next/previous node
public function focus-next-node();
public function focus-previous-node();

/// Select the currently focused node
public function select-focused-node(shift-held: bool);

/// Move focused/selected node(s) by delta
public function nudge-selection(delta-x: length, delta-y: length);
```

**2b. Recommended keymap (documented, embedder implements):**

| Key | Action |
|-----|--------|
| Tab | Focus next node |
| Shift+Tab | Focus previous node |
| Enter/Space | Select focused node |
| Shift+Enter | Add focused node to selection |
| Arrow keys | Nudge selected node(s) by grid step |
| Delete | Delete selected (already exists) |
| Escape | Cancel link creation / deselect |

The library documents this as the recommended pattern. The advanced example
demonstrates it.

**Files:** `node-editor.slint` (functions), advanced example (keymap demo)

---

### Tier 3: Pin navigation and keyboard link creation (library)

**Goal:** Pins are keyboard-navigable and links can be created without a mouse.

**3a. Pin focus within a node**

When a node has focus, a second level of navigation allows cycling through
its pins. This could be:
- Arrow Left/Right to cycle pins within the focused node
- Or a sub-focus mode entered with Enter, exited with Escape

Recommendation: When a node is focused, Left/Right arrow keys cycle through
pins. The focused pin is highlighted. This stays within the library since
it's structural navigation.

**3b. Keyboard link creation**

The flow:
1. Focus a pin (via node focus + arrow key)
2. Press Enter to start a connection from that pin
3. NodeEditor enters "keyboard link creation" mode
4. Tab navigates through compatible target pins (filtered)
5. Enter completes the connection; Escape cancels

This requires:
- A `keyboard-link-source-pin: int` property
- A `is-keyboard-linking: bool` state
- Modified Tab behavior during keyboard linking
- A way to enumerate and filter compatible pins

**3c. Pin accessible properties**

```slint
// In Pin component
accessible-description: /* "connected to Node B" or "not connected" */;
accessible-action-default => { /* start/complete link creation */ }
```

**Files:** `node-editor-building-blocks.slint` (Pin focus), `node-editor.slint` (link creation mode)

---

### Tier 4: Connection/Link accessibility

**Goal:** Links are discoverable by assistive technology.

**4a. Link accessible metadata**

Links are currently Path elements with no accessibility. Options:
- Add accessible-role to Link (but Path elements may not support it well)
- Maintain a separate accessible-only model of connections
- Describe connections in node/pin descriptions instead

Recommendation: Describe connections in pin descriptions since Slint doesn't
support accessible relationships. Each Pin's `accessible-description` would
include its connection targets:

```
"output pin, connected to Node B input"
"input pin, connected from Node A output"
```

This requires the library to track connections and generate descriptions.
Currently connections are opaque to the Slint side (only Rust knows pin
topology). A new global or callback would expose connection descriptions.

**4b. Node connection summary**

BaseNode's `accessible-description` could include a connection summary:
```
"selected, 2 connections: output to Node B, output to Node C"
```

This also requires Rust-side connection topology to be exposed to Slint.

**Files:** `node-editor-building-blocks.slint`, new global or callback for
connection descriptions, Rust helpers for generating descriptions

---

### Tier 5: Dynamic announcements (needs Slint extension)

**Goal:** State changes are announced to screen readers.

**Problem:** Slint has no live region support. There's no way to make a
screen reader announce "Node A selected" or "Connection created from A to B"
without the user navigating to the element.

**Workarounds within current Slint:**
- Use `accessible-value` on NodeEditor to hold a "last action" string that
  changes on every operation. Screen readers that watch value changes would
  pick this up. But this is unreliable.
- Use a hidden Text element with changing text — screen readers may announce
  text changes on focused elements.

**Slint extension needed:**
- `accessible-live: "polite" | "assertive"` on elements whose text changes
  should be announced without focus
- Or a `accessible-announce(message: string)` callback/function

Recommendation: File a Slint feature request for live region support. In the
meantime, implement the `accessible-value` workaround on NodeEditor to hold
a status string. This won't work on all platforms but is better than nothing.

---

## Implementation order

| Phase | Scope | Effort | Impact |
|-------|-------|--------|--------|
| **Tier 1** | Node focus + navigation | Medium | High — unblocks all keyboard use |
| **Tier 2** | Node operations via keyboard | Small | High — basic editing without mouse |
| **Tier 3** | Pin navigation + keyboard linking | Large | Medium — advanced operation |
| **Tier 4** | Connection descriptions | Medium | Medium — information for screen readers |
| **Tier 5** | Announcements | Small (workaround) | Low until Slint adds live regions |

Recommend implementing Tiers 1-2 first as they provide the most value.
Tier 3 is the most complex but essential for full keyboard operability.
Tier 4 provides information richness. Tier 5 depends on Slint extending
its accessibility facilities.

## Files to modify

| File | Changes |
|------|---------|
| `node-editor-building-blocks.slint` | BaseNode: focus support, focus ring, item properties. Pin: description with connections, focus within node |
| `node-editor.slint` | NodeEditor: focus tracking, Tab navigation functions, keyboard link creation mode, accessible-value for announcements |
| `src/lib.rs` or new module | Helpers for generating connection descriptions |
| `examples/advanced/ui/ui.slint` | Demonstrate recommended accessible keymap |
| `AGENTS.md` / `README.md` | Document accessibility features and recommended patterns |

## Slint extension requests

1. **Live regions** — `accessible-live: "polite"` for announcing state changes
2. **Accessible relationships** — `accessible-flowto` or equivalent for expressing
   node connections in the accessibility tree
3. **Repeater index exposure** — make repeater index available as a property for
   `accessible-item-index` without requiring manual plumbing

## Verification

- Screen reader testing (VoiceOver on macOS): navigate nodes, read labels,
  verify selection announcements
- Keyboard-only testing: Tab through nodes, select, delete, create links
- MCP introspection: verify accessible tree structure, labels, descriptions
- Automated: test accessible properties in level8 test file

## Reference implementations

- **React Flow**: Best-in-class web node editor accessibility
  - Tab navigation, Enter/Space select, Arrow key move
  - aria-live announcements
  - Semantic ARIA roles
  - https://reactflow.dev/learn/advanced-use/accessibility
- **Slint TabWidget**: Pattern for accessible-delegate-focus with child FocusScopes
  - Source: `slint-ui/slint/internal/compiler/widgets/*/tabwidget.slint`
