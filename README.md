# lazy-markdown

Goal: An efficient, safe, modular markdown editor with all dependencies written in Rust.

## Status

This is currently a plain text notepad, with no markdown functionality (peak laziness).

It currently relies on [a fork of floem](https://github.com/Third-Thing/floem/tree/dirty-tracking-and-stale-cursor-crash-fix) for proper dirty state tracking and a few crash fixes, while waiting for the [PRs](https://github.com/lapce/floem/pulls) to get merged.

### Current Features

- floem's built-in undo/redo
- floem's built-in save/open dialogs
- floem's built-in gutter (line numbers)
- tabs with per-tab dirty state
- menu actions and hotkeys for new/save/save-as/open
- custom save or discard on tab/window close overlay
- atomic writing

## Primary planned features

### v0.3.0

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

## License

[Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0)

As far as the law allows, this software comes as is,
without any warranty or condition, and no contributor
will be liable to anyone for any damages related to this
software or this license, under any kind of legal claim.
