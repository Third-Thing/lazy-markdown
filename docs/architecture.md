# Architecture

`lazy-markdown` is a small GPUI Component desktop application.

It is a plain text editor first, with markdown-aware structure tools as the
intended direction. It is not aiming to become an in-app markdown renderer or
preview application.

## Design Goals

The architecture is shaped by a few simple rules:

- Improve editing of source text directly rather than building duplicate rendered views.
- Keep app logic close to the UI workflows that trigger it.
- Use a small number of central state types instead of spreading behavior across many layers.
- Keep persistence and file helpers thin.
- Use GPUI Component directly instead of hiding it behind a toolkit-neutral layer.

## Project Layout

The crate is split by feature and support role:

- `src/main.rs` owns GPUI startup, window creation, and shared action types.
- `src/window.rs` owns top-level window state, action handlers, runtime theme switching, and window-close handling.
- `src/view.rs` owns the main GPUI render tree.
- `src/documents.rs` owns document IDs, per-document state, file open/save flows, tab activation, dirty tracking, and close confirmation.
- `src/menus.rs` installs key bindings and builds the GPUI app menu model.
- `src/preferences.rs` holds editor font family and size helpers.
- `src/persistence.rs` holds config, custom theme, recent-file storage, storage paths, and atomic write helpers.
- `pkg/` holds Linux package assets, currently the desktop file and hicolor icon used by `cargo-generate-rpm` and the window icon.

Feature modules keep focused `#[cfg(test)]` coverage nearby.

## Runtime Shape

The app is built around one GPUI window wrapped in `gpui_component::Root`.
The visible app surface has:

1. a top app menu bar
2. a document tab strip
3. the active multiline editor
4. a status strip
5. the GPUI Component dialog layer

`AppWindow` is the central state object. It stores the document list, active
document ID, recent files, app config, fontconfig-resolved editor font
families, status text, menu bar entity, and GPUI subscriptions.

Each `Document` owns:

- a stable `DocumentId`
- a GPUI Component `InputState`
- an optional current file path
- a pristine text snapshot
- a dirty flag

The editor surface defaults to GPUI Component's Markdown code-editor mode:
`InputState::code_editor("markdown")`. This keeps editing source text as the
main workflow while enabling Tree-sitter highlighting for Markdown structure.
Users can opt into GPUI Component's basic multiline editor by setting
`editor_mode = "basic"` in `config.toml`; the default config value is
`editor_mode = "code_editor"`.
The `gpui-component` dependency enables its `tree-sitter-markdown` feature so
the Markdown and Markdown inline parsers are registered for code-editor mode.
The app applies a small local theme adjustment before activating GPUI themes.
Missing highlight styles in custom themes are filled from the matching default
GPUI Component theme so partial custom highlight blocks keep Markdown styles
such as headings, links, italic emphasis, and strong emphasis.

## Startup

Startup happens in `src/main.rs`:

1. create a GPUI platform application with bundled GPUI Component assets
2. initialize GPUI Component
3. force GPUI Component scrollbars to stay visible
4. create the window with the `lazy-markdown` app ID and embedded icon image
5. create `AppWindow`
6. wrap it in `Root`
7. install window-close interception

`AppWindow::new` loads recent files and app config, applies the startup theme,
installs app menus, creates the menu bar entity, and opens the startup
document.

If the first CLI argument is a file path, the app reads that path into the
initial tab and records it in recent files. If no path is provided, it starts
with a blank untitled document. If the path cannot be read, it still starts
with a blank untitled document and shows the failure in the status strip.

## Documents And Tabs

Document operations live in `src/documents.rs`.

- `New` creates and activates a fresh untitled tab.
- `Open` opens a file dialog, reads the selected file, and records it in recent files.
- Opening a path that is already open activates the existing tab instead of opening a duplicate.
- `Save` writes the active document to its current path or falls through to Save As for untitled documents.
- `Save As` opens a save dialog and writes through `atomic-write-file`.
- Closing a dirty tab opens a discard confirmation dialog.
- Closing the final clean tab clears it back to a blank untitled document.
- Opening or creating more than five tabs shows a tab-limit dialog.

Dirty tracking compares the current editor value with the document's pristine
text snapshot. Saves update the snapshot and clear the dirty flag.

## Menus And Actions

GPUI actions are defined in `src/main.rs` and handled by listeners attached in
`src/view.rs`.

The app installs these key bindings:

- `Ctrl+N` for New
- `Ctrl+O` for Open
- `Ctrl+S` for Save
- `Ctrl+Shift+S` for Save As
- `Ctrl+=`, `Ctrl++`, `Ctrl+Shift+=`, and `Ctrl+Add` for zoom in
- `Ctrl+-` and `Ctrl+Subtract` for zoom out
- `Ctrl+0` for reset font size

The menu bar uses GPUI's app menu model plus `gpui_component::menu::AppMenuBar`.
The app rebuilds menus after recent-file, theme, and font changes so visible
menu state follows persisted state.

## Preferences And Persistence

User data is stored under the app's platform config and data directories.

- `config.toml` stores GPUI theme choice, editor mode, editor font family, and editor font size.
- `theme.json` can provide a custom GPUI Component theme.
- `recent-files.txt` stores the recent file list.

Writes use `atomic-write-file` for documents, config, and recent files.

The font menu stores generic choices for System, Sans Serif, Serif, and
Monospace. At startup, those choices are resolved through fontconfig generic
families: `system-ui`, `sans-serif`, `serif`, and `monospace`. If fontconfig
cannot resolve a family, rendering falls back to the current GPUI theme font or
mono font.

The editor render style sets an explicit normal font weight and a relative line
height. GPUI Component's `Input` default line height is rem-based, so using a
relative line height keeps vertical spacing tied to the selected editor font
size when zooming.

## Themes

The Theme menu supports:

- Default Light
- Default Dark
- Custom Theme

Default themes are applied from `ThemeRegistry`. Custom themes are read from
`theme.json`, parsed as a GPUI Component `ThemeSet`, and applied at runtime.

## Dialogs

The app uses GPUI Component dialogs for:

- dirty tab close confirmation
- dirty window close confirmation
- tab-limit messages

`Root::render_dialog_layer` is part of the render tree in `src/view.rs`; without
that layer, dialog state would open but not appear.

The window close hook uses `window.on_window_should_close`. If any documents are
dirty, the app activates the first dirty document, opens a confirmation dialog,
and returns `false` to cancel the platform close request. Confirming the dialog
calls `window.remove_window()`.
