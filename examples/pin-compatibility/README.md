# Pin Compatibility Example

This example demonstrates how to implement a robust validation system for connecting nodes with different data types. It shows how to enforce type safety in your node editor, preventing users from creating invalid connections (e.g., connecting a String output to a Boolean input).

## Key Concepts

### 1. Type Encoding
In this example, pin IDs carry type information. The application encodes pin IDs as:
`Pin ID = (Base ID) + (Type ID)`

Where:
-   **Source Node (Outputs):** Base ID = 100
-   **Sink Node (Inputs):** Base ID = 200
-   **Type IDs:**
    -   0: Execute (Flow control)
    -   1: Integer
    -   2: Float
    -   3: String
    -   4: Boolean
    -   5: Object
    -   6: Array
    -   7: Any

This allows the backend to instantly determine a pin's data type just by looking at its ID (`pin_id % 100`).

### 2. Custom Validation Logic
The core logic resides in the `TypeCompatibilityValidator` struct which implements the `LinkValidator` trait. It enforces a compatibility matrix:

| Source Type | Compatible Targets |
| :--- | :--- |
| **Execute** | Execute |
| **Integer** | Integer, Float, String, Any |
| **Float** | Float, String, Any |
| **String** | String, Any |
| **Boolean** | Boolean, Integer, Any |
| **Object** | Object, Any |
| **Array** | Array, Any |
| **Any** | All types |

### 3. Visual Feedback
The UI provides immediate feedback during link creation:
-   **Color Coding:** Pins and links are colored by type (e.g., Red for Boolean, Cyan for Integer).
-   **Validation Indicators:** A green checkmark appears on compatible target pins when dragging a link, guiding the user to valid connections.

## Code Highlights

### Rust Backend
-   **`examples/pin-compatibility/src/main.rs`**:
    -   `data_types` module defines the type constants.
    -   `types_compatible` function implements the matrix logic.
    -   `TypeCompatibilityValidator` struct plugs into the `NodeEditorController`'s validation pipeline.

### Slint UI
-   **`examples/pin-compatibility/ui/pin-compatibility.slint`**:
    -   `ValidatedPin` component adds the green checkmark overlay.
    -   `MainWindow` handles the `valid-target-pin-id` logic to update the UI state based on the drag operation.

## Running the Example

```bash
cargo run -p pin-compatibility
```
