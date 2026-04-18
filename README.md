# lazy-markdown

Goal: An efficient, safe, modular plain text editor with markdown-aware structure tools, with all dependencies written in Rust.

The basic principle for this project is that minor usability conveniences don't outweigh gross programmatic inefficiencies. For example, almost nothing will be triggered on every keystroke.

## Scope

`lazy-markdown` is a plain text editor first.

The project aims to help people edit markdown source more effectively through structure-aware tools, not by maintaining a second rendered view of the document.

That means the project is interested in features such as:

- outline navigation
- heading-aware movement and selection
- markdown normalization
- structural transforms
- source cleanup and conversion helpers

It is not interested in features such as:

- split view
- live preview
- built-in rendered markdown views
- always-on background work that duplicates what the source text already says

## Status

This is currently a plain text tabbed editor. The implementation strategy is to build a robust plain text editor first, then add markdown-aware editing features that improve source editing directly.

It currently relies on [a fork of floem](https://github.com/Third-Thing/floem/tree/dirty-tracking-and-stale-cursor-crash-fix) for proper dirty state tracking and a few crash fixes, while waiting for the [PRs](https://github.com/lapce/floem/pulls) to get merged.

The repo also tracks concrete Floem friction found in real app code in [docs/floem-pain-points.md](docs/floem-pain-points.md).

This program is only tested on Linux/Wayland.

### Current Features

- floem's built-in undo/redo
- floem's built-in save/open dialogs
- floem's built-in gutter (line numbers)
- tabs with per-tab dirty state
- menu actions and hotkeys for new/save/save-as/open
- editor font family picker with per-user persistence
- standard zoom hotkeys for editor font size: `Ctrl+=`, `Ctrl+-`, `Ctrl+0`
- recent documents menu with per-user persistence
- custom save or discard on tab/window close overlay
- atomic writing
- dark/light theme options

## Planned features

### v0.5.0

- folder view

### Beyond

- outline navigation
- heading-aware document navigation
- search / replace
- spell check
- configuration for hotkeys
- structural markdown transforms
- markdown normalization
- convert pasted content to markdown
- optional markdown parsing for structure-aware commands
- LSP / service API for iwe, quickmark, etc

## License

[Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0)

As far as the law allows, this software comes as is,
without any warranty or condition, and no contributor
will be liable to anyone for any damages related to this
software or this license, under any kind of legal claim.
