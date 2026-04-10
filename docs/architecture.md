# Architecture

`lazy-markdown` is a Cargo workspace split into a small core crate and a Floem app crate.

The split keeps reusable command descriptions in `core` while the app owns editor behavior, saving, dialogs, and UI state.

## How It Works

The app is the host. It owns the window, the editor, the current file path, the unsaved-change state, file dialogs, saves, and status messages.

The core crate is the shared rulebook for commands. It defines command IDs, command titles, default shortcuts, and placement hints for UI surfaces such as a toolbar or command palette.

At startup, the host builds its command list:

1. It creates a module registry.
2. It adds the built-in file commands.
3. It starts the editor window with the command registry.

After startup, user actions go through commands. A toolbar click and a keyboard shortcut both ask the host to run the same command ID. For example, clicking Save and pressing the Save shortcut both run `file.save`.

When `file.save` runs, the host checks whether the current document already has a file path. If it does, the host writes the current text to that path. If it does not, the host opens a Save As dialog first, then writes to the selected path.

Saves are atomic. The app writes the editor text into a temporary file, then commits the temporary file over the destination path after the write succeeds. The editor text is written from rope chunks through a buffered writer, so saving does not first build one full `String` copy of the document.

## Project Layout

- `crates/core` defines command metadata and command/module registries. It should stay independent from Floem where practical.
- `crates/app` contains the Floem UI, startup wiring, command dispatch, dialog handling, editor state, document flow, and atomic save implementation.

## Core

`core` currently owns the contracts that command providers and the app share:

- `CommandRegistry` stores command metadata by stable command ID.
- `ModuleRegistry` groups command registration so future modules can register commands through one host-facing entry point.

The built-in file commands currently live in `core`:

- `file.new`
- `file.open`
- `file.save`
- `file.save_as`

Each command has a stable ID, title, optional default shortcut, and placement hints for UI surfaces such as toolbar and palette.

## App Host

`app` owns UI state and user interaction:

- Floem views and styles.
- File open and save dialogs.
- Current file path.
- Dirty and pristine document updates.
- Pending open, close, and new-document flows.
- Status messages.
- Shortcut translation.
- Command dispatch.
- Atomic saving.

Commands are the shared entry point for UI actions. The toolbar and keyboard handler both call `invoke_command` with a command ID, so `file.save` and `file.save_as` are not separate UI-only code paths.

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

## Current Limits

Toolbar command order is still host-owned through a fixed list of command IDs. The registry can filter by placement, but the command metadata does not yet include ordering or grouping.

Before fully generating the toolbar from the registry, add explicit metadata for order and groups so the UI is stable and intentional.

The future service boundary for out-of-process tools should be added after the native command model settles. Good candidates are linting, project search, and document analysis. Save and deep editor behavior should remain native host code unless there is a clear reason to move them elsewhere.
