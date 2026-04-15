# Architecture

`lazy-markdown` is a single Cargo crate with a small command-model layer and a Floem host in the same source tree.

The command model stays separate at the module level so command descriptions remain independent from the UI code, while the host owns editor behavior, saving, dialogs, and UI state.

## How It Works

The app host owns the window, the document set, editor state, file dialogs, saves, and status messages.

The command model is the shared rulebook for commands. It defines command IDs, command titles, default shortcuts, and placement hints for UI surfaces such as a menu or command palette.

At startup, the host builds its command list:

1. It creates a module registry.
2. It adds the built-in file and view commands.
3. It starts the editor window with the command registry.

After startup, user actions go through commands. A menu item and a keyboard shortcut both ask the host to run the same command ID. For example, choosing Save and pressing the Save shortcut both run `file.save`.

When `file.save` runs, the host checks whether the active document already has a file path. If it does, the host writes that document's text to the path. If it does not, the host opens a Save As dialog first, then writes to the selected path.

`file.new` creates a new tab with its own editor and document state. `file.open` opens the selected file in a new tab, or activates the existing tab if that path is already open.

Saves are atomic. The app writes the editor text into a temporary file, then commits the temporary file over the destination path after the write succeeds. The editor text is written from rope chunks through a buffered writer, so saving does not first build one full `String` copy of the document.

## Project Layout

- `src/commands.rs` defines command metadata and the command registry. It should stay independent from Floem where practical.
- `src/state.rs` holds app and document state.
- `src/documents.rs` owns document lifecycle and file read/write flows.
- `src/shortcuts.rs` maps keypresses to commands.
- `src/views/` contains Floem view code grouped by UI area.
- `src/main.rs` handles startup and app composition.

## Command Model

`commands.rs` currently owns the contracts that command providers and the app share:

- `CommandRegistry` stores command metadata by stable command ID.
- `ModuleRegistry` groups command registration so future modules can register commands through one host-facing entry point.

The built-in file commands currently live in `commands.rs`:

- `file.new`
- `file.open`
- `file.save`
- `file.save_as`

Each command has a stable ID, title, zero or more default shortcuts, and placement hints for UI surfaces such as menu and palette.

Shortcuts can match either the logical key value, such as `s`, or the physical key code, such as `Equal` or `Digit0`. Physical codes are useful for standard shortcuts like zoom where the same keyboard position should work across layouts and shifted variants.

## Host

The host owns UI state and user interaction:

- Floem views and styles.
- File open and save dialogs.
- Document set state, including per-tab editor and file path data.
- Dirty and pristine document updates.
- Pending close-tab and close-window flows.
- Status messages.
- Shortcut translation.
- Command dispatch.
- Atomic saving.

Commands are the shared entry point for UI actions. The menu and keyboard handler both call `invoke_command` with a command ID, so `file.save` and `file.save_as` are not separate UI-only code paths.

This keeps later UI surfaces, such as context menus or a command palette, able to use the same command IDs and metadata.

## Save Flow

Save behavior is intentionally not a separate module right now.

The app saves directly because atomic save is the safe default, the dependency is small, and the previous configurable save backend added more complexity than value.

The save path is:

1. Get the editor rope.
2. Open an atomic temporary file for the destination path.
3. Write rope chunks into a buffered writer.
4. Flush and finish the buffered writer.
5. Commit the atomic file.
6. Mark the document pristine and update the current path/status message.

The app keeps ownership of all user-facing save behavior. That includes Save As dialogs, pending close/open flow, dirty/pristine updates, and status messages.

Document-owned state is separate from app/window state. `DocumentState` stores a document ID, file path, and editor. `DocumentSet` stores the tabs, active document ID, and next document ID. `AppState` stores the document set plus window-level status, pending actions, confirmation overlay state, dialog flags, and the scope used to create new editors.

## Current Limits

Menu command order is still host-owned through fixed command IDs. The registry can filter by placement, but the command metadata does not yet include ordering or grouping.

Before fully generating the menu from the registry, add explicit metadata for order and groups so the UI is stable and intentional.

The future service boundary for out-of-process tools should be added after the native command model settles. Good candidates are linting, project search, and document analysis. Save and deep editor behavior should remain native host code unless there is a clear reason to move them elsewhere.
