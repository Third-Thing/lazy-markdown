# Architecture

`lazy-markdown` is a single Cargo crate. The app keeps its command registry, document model, Floem window, and local persistence in one source tree.

The code is split by role at the module level:

- `src/bootstrap.rs` builds startup data that must exist before the window opens.
- `src/commands.rs` defines command metadata and the built-in command list.
- `src/app_keys.rs` and `src/shortcuts.rs` turn key events into either menu actions or command IDs.
- `src/state.rs` and `src/documents.rs` own document state, tab state, save flow, and close flow.
- `src/views/` contains Floem views for menus, tabs, the editor area, and modal dialogs.
- `src/config.rs`, `src/recent_files.rs`, `src/theme.rs`, `src/editor_font.rs`, and `src/paths.rs` handle user settings, recent file history, look and feel, and per-platform storage paths.
- `src/main.rs` wires those pieces together into one application window.

## Runtime Shape

The host is responsible for almost all runtime behavior:

- window creation
- top bar, tab strip, editor area, status strip, and confirm overlay
- document creation, activation, and closing
- open and save dialogs
- atomic file writes
- dirty tracking and close confirmation
- recent file history
- theme and editor font preferences
- status messages

The command layer is smaller. It gives the app stable command IDs, labels, default shortcuts, and placement hints. The host then uses those IDs from menus and keyboard handling.

That boundary is useful but not fully detached yet. `CommandRegistry` is clean metadata, but `invoke_command` still lives in `src/commands.rs` and dispatches straight into `AppState`, document code, and font-size code. In practice, the app has a command metadata layer plus a host-owned command executor, not a fully separate command subsystem.

## Startup

Startup happens in two steps.

First, `AppBootstrap::load`:

1. creates a `ModuleRegistry`
2. registers the built-in commands
3. loads `AppConfig`, falling back to defaults if the config file cannot be read or parsed

Second, `app_view` in `src/main.rs`:

1. loads the recent file list, again falling back to an empty list on error
2. creates `AppState` with the current Floem scope, recent files, and config
3. syncs the window theme from the saved theme preference
4. opens the first CLI argument as the initial document if one was provided
5. otherwise creates a blank untitled document
6. records the opened path in recent files when the initial document came from disk
7. builds the window view tree and attaches app-level event handlers

Config and recent-file load failures are not fatal once the process has started. They are surfaced through the status message strip instead.

## Core State

There are three main state types.

- `DocumentState` stores a stable `DocumentId`, an optional file path signal, and a Floem `Editor`.
- `DocumentSet` stores the open tabs, the active document ID, and the next document ID to allocate.
- `AppState` stores the document set plus window-level state such as menu state, recent files, pending actions, the confirm overlay flag, the Save As dialog guard, user config, theme state, and the shared scope used to create new editors.

`DocumentSet` also owns a few cross-tab rules:

- activate a tab by `DocumentId`
- remove a tab and pick the next active tab
- find an already-open document by path
- collect dirty document IDs for window-close handling

The path lookup normalizes paths through `save_target_path`, so the app can treat different spellings of the same file path as one open document.

## Commands And Input

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
- zero or more default shortcuts
- placement hints such as `Menu` or `Palette`

Keyboard handling works in two layers.

`src/app_keys.rs` handles app-wide key capture. It first checks for top-level menu shortcuts:

- `Alt+F` opens or closes the File menu
- `Alt+R` opens or closes the Recent menu
- `Alt+T` opens or closes the Theme menu
- `Alt+O` opens or closes the Font menu

If a menu is open, arrow keys, Enter, and Escape are routed to menu navigation. If no menu is open, `src/shortcuts.rs` scans the command registry and matches the key event against command shortcuts.

Shortcuts can match either:

- the logical key value, such as `s`
- the physical key code, such as `Equal`, `Minus`, or `Digit0`

The physical-code path is used for zoom shortcuts so standard key positions still work across keyboard layouts and shifted variants.

## Menus And UI Composition

The window view tree is assembled in `src/main.rs`:

1. a top bar with the menu bar
2. a tab strip
3. the active editor view
4. a status strip
5. a confirm overlay layered above the main content

`src/views/menu.rs` builds four top-level menus:

- File
- Recent
- Theme
- Font

Only the File menu is built from command metadata today. The Recent, Theme, and Font menus are still host-owned view models.

This means the registry already helps with command labels and shortcuts, but menu structure is still partly hard-coded in the view layer.

## Document Flow

`src/documents.rs` owns document lifecycle.

`file.new` creates a fresh editor tab with no file path. `file.open` opens a Floem file dialog, reads the selected file into a new tab, and records that path in recent files. If the chosen path is already open, the app activates the existing tab instead of opening a duplicate.

The host currently enforces a hard cap of five open tabs. When the user tries to open a new file or create a new tab past that limit, the app shows a modal message instead of opening another document.

Close behavior is also host-owned:

- closing a dirty tab starts a confirm flow
- closing the window walks through dirty documents one at a time
- closing the last remaining tab does not leave the app empty; it resets that tab to a fresh untitled document

Pending close work is tracked through `PendingAction`, which lets the same confirm overlay handle close-tab, close-window, and simple message dialogs.

## Save Flow

Saving is handled directly in `src/documents.rs`.

When `file.save` runs, the host checks whether the active document already has a file path:

- if it does, the app saves straight to that path
- if it does not, the app opens a Save As dialog first

The write path is:

1. resolve the destination path
2. open an atomic temp file with `atomic-write-file`
3. stream the editor rope into a buffered writer chunk by chunk
4. flush the writer
5. commit the temp file over the destination
6. mark the document pristine
7. update the document path, recent files, and status message

The app writes rope chunks directly, so it does not build one large `String` copy just to save a document.

## Persistence And Preferences

The app stores two kinds of user data outside the project tree.

- `config.toml` in the platform config directory
- `recent-files.txt` in the platform data directory

`src/paths.rs` picks the base directories per platform:

- Windows uses `APPDATA` or `LOCALAPPDATA`
- macOS uses `~/Library/Application Support`
- Linux and other Unix-like targets use XDG paths when present, then fall back to `~/.config` and `~/.local/share`

`AppConfig` currently stores:

- theme preference
- editor font family
- editor font size

Config values are normalized on load. Unknown font names fall back to the system default option, and font size is clamped to the supported range.

Recent files are also normalized and deduplicated by resolved path. The list is capped at ten entries.

Theme handling is split between saved preference and live OS state:

- `Light` and `Dark` force a specific Floem theme
- `FollowOs` listens for Floem theme changes and keeps the app in sync with the window system theme

The app saves config on window exit. Recent files are persisted immediately when the list changes. Both use atomic writes.

## Current Limits

The architecture is still intentionally small, but a few limits are clear in the current code.

- The command registry knows about placement hints, but there is no command palette or context menu yet. `Palette` is future-facing metadata right now.
- Menu grouping and order are still host-owned in `src/views/menu.rs` and `src/state.rs`, so the registry is not yet rich enough to generate the full menu bar on its own.
- Command execution is still tied to `AppState` and host modules, so the command layer is not yet a clean standalone boundary.
- Theme changes and editor font family changes are host-owned actions rather than registry-backed commands.
- The tab limit is a fixed constant in document code rather than a user setting.

If the app grows further, the next useful split is likely richer command metadata and a cleaner boundary between command description and command execution. Save flow, close confirmation, and deep editor behavior still fit best in the host because they are tightly tied to UI state and document state.

For toolkit-specific rough edges found while building these flows, see [floem-pain-points.md](./floem-pain-points.md).
