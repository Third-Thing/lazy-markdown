# lazy-markdown

Goal: An efficient, safe, modular markdown editor with all dependencies written in Rust.

## Primary planned features (or bundled plugins)

### v0.3.0

- tabs
- recent documents list

### Beyond

- lazy markdown parsing (manual trigger or debounce)
- LSP / service API for iwe, quickmark, etc
- configurable markdown normalization
- convert pasted content to markdown
- a secondary window for preview / rendering
- folder view
- search / replace
- spell check
- configuration file for style and hotkeys

## Status

This is currently a plain text notepad, with no markdown functionality (peak laziness).

It currently relies on a commit that's [waiting to be merged into floem](https://github.com/lapce/floem/pull/1060) for proper dirty state tracking.

### Current Features

- floem's built-in undo/redo
- floem's built-in save/open dialogs
- floem's built-in gutter (line numbers)
- buttons for new/save/save-as/open
- custom Ctrl+S hotkey
- custom save or discard on close overlay
- atomic writing

Buttons are used instead of a menu because floem doesn't provide a menu for linux yet.

## License

[Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0)

As far as the law allows, this software comes as is,
without any warranty or condition, and no contributor
will be liable to anyone for any damages related to this
software or this license, under any kind of legal claim.