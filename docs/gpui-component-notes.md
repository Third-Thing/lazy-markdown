# GPUI Component Notes

These notes record what we learned while building and promoting the GPUI
Component version of `lazy-markdown`.

## Current GPUI Shape

The GPUI app is the root crate. It is split into small files:

- `src/main.rs` owns GPUI startup, window creation, and the shared action types.
- `src/documents.rs` holds document IDs, per-document state, document display helpers, and document workflows.
- `src/menus.rs` installs key bindings and builds the GPUI app menu model.
- `src/persistence.rs` stores recent files in `recent-files.txt` and config in
  `config.toml`.
- `src/view.rs` owns the main GPUI render tree.
- `src/window.rs` owns top-level GPUI window state and action handlers.

It currently validates:

- a multiline text editor surface
- startup from the first CLI file path, falling back to a blank untitled document
- normal edit hotkeys and the default input context menu
- app-owned document tabs rendered with `gpui_component::tab::TabBar`
- a visible app menu bar with File actions
- a visible Recent menu backed by the persisted recent-file list
- a visible Theme menu that switches between GPUI Component's bundled
  `Default Light` and `Default Dark` themes plus a user custom theme at
  runtime
- Open, Save, and Save As file-dialog flows
- New document handling as a new tab
- dirty tracking against a pristine text snapshot per tab
- tab close confirmation for dirty documents
- dirty-document confirmation when the platform asks to close the window

## Tabs

The GPUI app now keeps a small document list directly on `AppWindow`. Each
`Document` owns its GPUI Component `InputState`, current file path,
pristine text snapshot, and dirty flag.

The tab strip is rendered with `TabBar` and `Tab` from `gpui-component`, not
through an app-specific wrapper. The render method maps the document list into
tabs, marks the selected index from the active document, and uses a close
`Button` as each tab suffix.

Tab behavior:

- `New` creates and activates a fresh untitled tab.
- `Open` activates an already-open path instead of opening a duplicate.
- opening or creating a sixth tab shows the five-tab limit
- closing a dirty tab opens an alert dialog before discarding changes
- closing the last clean tab clears it back to a blank untitled document

Focus stays direct: activating a tab stores the active document ID and then
focuses that document's `InputState` with its GPUI focus handle.

## Input, Code Editor, and Tree-sitter

`gpui-component` uses `Input` plus `InputState` for this editor surface. The
code-editor behavior is not a separate widget in the GPUI app; it is a mode on
`InputState`.

The GPUI app uses Markdown code-editor mode:

```rust
InputState::new(window, cx)
    .code_editor("markdown")
    .searchable(true)
    .default_value(text)
```

The `gpui-component` dependency must enable `tree-sitter-languages`; without
that feature, `code_editor("markdown")` has no registered Markdown parser and
does not produce Markdown highlights.

Markdown headings are colored because:

- `/my/src/gpui-component/crates/ui/src/highlighter/languages/markdown/highlights.scm`
  captures Markdown headings as `@title`.
- `/my/src/gpui-component/crates/ui/src/theme/default-theme.json` maps
  `syntax.title` to a blue color.

That means the blue heading color is not a one-off Markdown rule in the editor
view. It comes from the normal highlight query plus theme mapping.

Markdown inline content is parsed through the Markdown injection query:

- `/my/src/gpui-component/crates/ui/src/highlighter/languages/markdown/injections.scm`
  maps `(inline)` content to `markdown_inline`.
- `/my/src/gpui-component/crates/ui/src/highlighter/languages/markdown_inline/highlights.scm`
  captures strong emphasis as `@emphasis.strong`.

The app fills missing highlight styles from the matching default GPUI Component
theme, then adds `font_weight = 700` for
`highlight.syntax.emphasis.strong` before applying a theme when the theme does
not already define a weight for that capture. This makes Markdown
strong-emphasis spans render bold while still letting custom themes provide
their own weight. While testing the Markdown inline highlight path, the app also
fills in a dark blue fallback color for that capture.

`InputState::code_editor("text")` removes visible Markdown colors, but it still
uses the code-editor path. It is not the same as bypassing the syntax system.

Plain multiline input bypasses the Tree-sitter highlight path:

- `set_value` only queues highlight updates when the mode is code editor.
- `highlight_lines` returns no highlight data unless the mode is code editor.
- `update_highlighter` only creates a `SyntaxHighlighter` in the code-editor
  mode branch.

Plain multiline mode also removes code-editor-specific UI such as line numbers
and code-editor context menu entries. Normal text editing behavior still works.

## Context Menus

Markdown code-editor input includes code-editor context menu entries.

The default input context menu can be replaced through
`Input::context_menu(...)`. Source inspection showed the callback receives an
empty `PopupMenu`, so this is a replacement point, not a direct "extend the
built-in menu" point.

If the converted app needs to add app-specific entries while keeping the default
entries, we should first confirm whether upstream has another supported helper.
If not, we will likely need to build the whole context menu ourselves.

## Menus and Hotkeys

The visible app menu bar comes from `AppMenuBar`, backed by the app menu model.

The GPUI app currently does both:

- `cx.set_menus(build_app_menus())`
- `GlobalState::global_mut(cx).set_app_menus(owned_menus)`

The second call feeds the visible `AppMenuBar`. The first call feeds GPUI's app
menu system.

When the recent-file list changes, the GPUI app rebuilds both menu stores and then
calls `AppMenuBar::reload`. Without that explicit reload, the visible menu bar
keeps showing the old Recent menu even though the stored app menu model has
changed.

The Theme menu uses `gpui_component::Theme::change` with `ThemeMode::Light` and
`ThemeMode::Dark` for the bundled defaults. After custom-theme testing, the
default menu entries were changed to apply `ThemeRegistry`'s default light and
dark configs directly, because applying a custom light theme replaces the
active `light_theme` used by `Theme::change(ThemeMode::Light)`. The menu also
has a `Custom Theme` entry that reads `theme.json` from the platform config
directory, parses it as a GPUI Component `ThemeSet`, and applies the first
theme in that file.

One earlier limitation showed up in plain input mode: light-mode `Input`
derived its background from `cx.theme().background`, so a custom theme that
changed only `background` changed the whole app background too. Markdown
code-editor mode can use GPUI Component's `editor.background` highlight
setting.

The selected GPUI theme is persisted as `gpui_theme` in `config.toml`. Menu
checkmarks are based on the persisted GPUI choice, so the menu model is
reloaded after a theme change.
The editor input now relies on the component's own themed styling instead of
setting a fixed background color in the GPUI view.

On Linux, explicit `ctrl-*` bindings were needed for expected shortcuts. The
initial `cmd-*` bindings mapped to the Windows key in this environment.

Recent-file entries use a payload action:

```rust
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = lazy_markdown, no_json)]
pub(crate) struct OpenRecent(pub(crate) String);
```

This lets `MenuItem::action` dispatch the selected path through the normal GPUI
action path.

## File Dialogs

GPUI provides file-dialog helpers directly on app/window context:

- `cx.prompt_for_paths(PathPromptOptions { ... })` for Open.
- `cx.prompt_for_new_path(&directory, Some(&suggested_name))` for Save As.

The prompt APIs return receivers, so the GPUI app starts an async task with
`cx.spawn_in(window, ...)`, awaits the selected path, then updates the view on
the window with `window.update(...)`.

The current file I/O is deliberately thin and local to the GPUI app. It reads UTF-8
text with `std::fs` and writes through `atomic-write-file`.

## Dialogs

`gpui-component` has a ready alert dialog path:

- import `WindowExt`
- call `window.open_alert_dialog(cx, |dialog, window, cx| { ... })`
- configure title, description, and `DialogButtonProps`
- return `true` from `on_ok` or `on_cancel` to close the dialog

One important detail: `Root::new(...)` stores dialog state, but it does not add
dialog UI to the render tree by itself. The app shell must render the layer:

```rust
let dialog_layer = Root::render_dialog_layer(window, cx);
...
.children(dialog_layer)
```

Without that layer, the dialog opens in state but nothing appears.

For dirty tab close actions, the GPUI app opens a discard dialog and only closes
the tab after the user confirms.

## Window Close Interception

GPUI exposes a cancellable window close hook:

```rust
window.on_window_should_close(cx, move |window, cx| {
    // return false to cancel close
    // return true to allow close
})
```

The GPUI app uses this to block platform close when the document is dirty. If the
document is clean, the callback returns `true`. If the document is dirty, it
opens a confirmation dialog and returns `false`.

When the user confirms the dialog, the GPUI app calls `window.remove_window()` to
close the window programmatically.

The close hook can be called repeatedly while a dialog is already open, so the
GPUI app checks `window.has_active_dialog(cx)` to avoid stacking duplicate close
dialogs.

## Conversion Notes

The main early finding is that GPUI Component gives us standard desktop
behavior with less app-owned workaround code:

- normal editing hotkeys are already present in plain multiline input
- the default text context menu works
- scroll bar spacing is correct without a local wrapper workaround
- app menus and dialogs are available, but they need explicit setup

The main caution is that GPUI Component is still explicit about app shell
layers. Menus, dialog layers, and close handling are not fully automatic just
because the app is wrapped in `Root`.

For `lazy-markdown`, plain multiline input is the better starting point than
code-editor mode until we intentionally want syntax coloring, line numbers,
language-aware context menu entries, or other code-editor behavior.
