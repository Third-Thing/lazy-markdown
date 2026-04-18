# Architecture

`lazy-markdown` is a small Floem desktop application.

It is a plain text editor first, with markdown-aware structure tools as the intended direction. It is not aiming to become an in-app markdown renderer or preview application.

The project is intentionally not trying to hide its GUI toolkit. The app keeps Floem-native state, Floem-native editor objects, and Floem-native event flow close to the features that use them. That makes the repo more useful as:

- a practical reference for structuring a real Floem app
- a place to record where Floem works well
- a place to record where Floem still makes normal desktop behavior awkward

For the current list of toolkit-specific rough edges, see [floem-pain-points.md](./floem-pain-points.md).

## Design Goals

The architecture is shaped by a few simple rules:

- Prefer the clearest Floem-native design over toolkit-neutral abstraction.
- Improve editing of source text directly rather than building duplicate rendered views.
- Keep app logic close to the UI workflows that trigger it.
- Use a small number of central state types instead of spreading behavior across many layers.
- Keep persistence and file helpers thin.
- Let the repo expose real Floem friction instead of hiding it behind wrapper code.

## Project Layout

The crate is split by feature and support role:

- `src/main.rs` builds the window, top-level view tree, and app-level event hooks.
- `src/bootstrap.rs` loads startup data before the Floem window opens.
- `src/commands.rs` defines command metadata and built-in command dispatch used by menus and shortcuts.
- `src/workspace/mod.rs` groups document, tab, editor-area, and workspace state code under one feature area.
- `src/workspace/state.rs` holds the core app state, document state, tab state, menu state, and pending modal actions.
- `src/workspace/documents.rs` owns document lifecycle, file open/save flows, tab activation, and close flows.
- `src/workspace/editor_area.rs` contains the active editor-area view.
- `src/workspace/tabs.rs` contains the tab strip UI.
- `src/menus/mod.rs` groups the app menu setup and re-exports the public menu entry points.
- `src/menus/model.rs` holds menu item and menu model types that are shared by menu UI and menu key handling.
- `src/menus/view.rs` contains the Floem menu bar and popup UI.
- `src/menus/keys.rs` handles app-level menu capture, popup navigation keys, and menu activation.
- `src/views/` currently contains dialog views.
- `src/shortcuts.rs` handles command shortcut matching when the menu system does not consume the keypress.
- `src/persistence/` holds config loading, recent-file storage, and per-platform storage paths.
- `src/preferences/` holds app theme behavior and editor font preferences.

## Runtime Shape

The app is built around one Floem window with a small set of app-owned surfaces:

1. a top bar with the menu bar
2. a tab strip
3. the active editor area
4. a status strip
5. a confirm overlay for save/discard and message dialogs

The same `AppState` instance is shared across those surfaces. Floem signals and effects keep the UI in sync with:

- the active document
- dirty and pristine status
- menu open state and selection state
- recent files
- theme preference
- editor font family and size
- status messages
- pending close or dialog actions

This is intentionally a Floem app with support modules, not a generic editor core with a Floem shell.

## Core State

Three state types carry most of the runtime model.

- `DocumentState` stores a stable `DocumentId`, an optional file path signal, and a Floem `Editor`.
- `DocumentSet` stores the open tabs, the active document ID, and the next document ID to allocate.
- `AppState` stores the document set plus window-level state such as menu state, popup IDs, recent files, pending actions, confirm visibility, dialog guards, config, and theme state.

This is an important architectural choice: document state owns Floem editor objects directly. The app does not try to wrap the editor in a toolkit-neutral model.

That keeps flows such as dirty tracking, focus, editor styling, and view creation simple, but it also means the architecture follows Floem's strengths and limits closely. The pain-points doc exists partly because of that directness.

## Startup

Startup happens in two stages.

First, `AppBootstrap::load`:

1. creates a module registry
2. registers built-in commands
3. loads `AppConfig`, falling back to defaults if config loading fails

Second, `app_view` in `src/main.rs`:

1. loads the recent file list
2. creates `AppState` with the current Floem scope, recent files, and config
3. syncs the starting theme from the saved preference and current window theme
4. opens the first CLI argument as the initial document if one was provided
5. otherwise creates a blank untitled document
6. records an opened startup path in recent files
7. builds the full view tree and attaches app-level event handlers

Config and recent-file load failures are shown in the status strip instead of aborting startup.

## Floem-Native Workflows

Several important workflows are deliberately handled in the view-and-state layer rather than being pushed behind abstraction boundaries.

### Menus and keyboard routing

The menu system is app-owned UI built under `src/menus/`.

The app currently has four top-level menus:

- File
- Recent
- Theme
- Font

Keyboard routing is split across:

- `src/menus/keys.rs` for app-level capture, top-level menu shortcuts, and popup navigation keys
- `src/menus/view.rs` for the Floem menu bar and popup surfaces
- `src/shortcuts.rs` for command shortcut matching when normal app content has focus

This split is part of the real Floem story of the app. Overlay behavior, focus, and routing details matter here, and the code keeps that visible rather than hiding it behind a generic event layer.

The current `menus/` layout also keeps one likely future contribution path visible: the reusable parts are the menu model, popup behavior, and keyboard routing, while the actual menu contents are still app-owned. That keeps Linux-specific focus and keyboard behavior easy to inspect instead of burying it in a general wrapper.

### Tabs and editor focus

Each tab owns a Floem `Editor`. Activating a tab updates app state first and then syncs focus in the rendered editor view once the matching editor view ID exists.

That is more direct than introducing a separate focus manager abstraction, and it reflects how Floem view creation and focus targets actually behave.

### Close confirmation

The confirm overlay is one reusable Floem surface that handles:

- closing a dirty tab
- closing a window with dirty documents
- simple app message dialogs such as the tab-limit warning

The current action is tracked through `PendingAction`, and the overlay reads that state to decide its title, message, buttons, and follow-up behavior.

### Theme and editor styling

The app uses Floem's window theme switching for light, dark, and follow-OS behavior, but it also keeps an app-owned theme layer for custom chrome such as:

- the top bar
- menu buttons and popups
- tab strip
- status strip
- dialog shell

Editor font family and size are also app-owned preferences. When they change, the app updates the styling of every open editor directly.

That code now lives under `src/preferences/`:

- `theme.rs` handles theme preference state, window-theme syncing, and editor theme styling
- `editor_font.rs` handles editor font options, size changes, and editor restyling

## Commands

Commands exist to support menus, shortcuts, and future command-oriented UI such as a palette. They are useful, but they are not the center of the architecture.

The built-in commands are:

- `file.new`
- `file.open`
- `file.save`
- `file.save_as`
- `view.zoom_in`
- `view.zoom_out`
- `view.zoom_reset`

Each command has:

- a stable ID
- a title
- default shortcuts
- placement hints such as `Menu` or `Palette`

The command registry helps the app avoid duplicating labels and shortcut definitions across menus and keyboard handling. Execution still stays close to the Floem app state and document flows, which is the right tradeoff for this repo's goals.

## Document and Save Flow

`src/workspace/documents.rs` owns document lifecycle.

`file.new` creates a fresh untitled tab. `file.open` opens a Floem file dialog, reads the selected file into a new tab, and records the path in recent files. If the selected path is already open, the app activates that existing tab instead of opening a duplicate.

The app currently enforces a hard cap of five open tabs. If the user tries to open more, the app shows a modal message through the same confirm overlay system.

Saving is also handled directly in `src/workspace/documents.rs`.

When `file.save` runs:

- if the active document already has a path, the app saves straight to that path
- if it does not, the app opens a Save As dialog first

The actual write path is:

1. resolve the destination path
2. open an atomic temp file
3. stream the editor rope into a buffered writer chunk by chunk
4. flush the writer
5. commit the temp file over the destination
6. mark the document pristine
7. update the document path, recent files, and status message

This stays close to the app because save behavior is tightly tied to dialogs, dirty tracking, close confirmation, and user-visible status updates.

## Persistence

The app stores two kinds of user data outside the project tree:

- `config.toml` in the platform config directory
- `recent-files.txt` in the platform data directory

That code now lives under `src/persistence/`:

- `config.rs` handles config parsing and atomic config writes
- `recent_files.rs` handles recent-file loading, normalization, and atomic writes
- `paths.rs` resolves platform-specific config and data directories

`AppConfig` currently stores:

- theme preference
- editor font family
- editor font size

Recent files are normalized and deduplicated by resolved path. Config and recent files both use atomic writes.

These modules are intentionally thin. They support the Floem app rather than define a separate architecture story of their own.

## Why This Structure

This structure is meant to keep the repo readable as a real Floem application.

The main value of the code base is not that it cleanly separates every concern from the toolkit. The main value is that someone can read it and see:

- how to structure a small Floem app
- how to manage app-owned menus, overlays, tabs, and editor state
- how to wire app-wide key handling
- how to combine persistence with reactive UI state
- where Floem currently creates extra work for app code

If the app grows, new features should continue to strengthen that role. Good additions are features that both improve the editor and teach a meaningful Floem pattern.
