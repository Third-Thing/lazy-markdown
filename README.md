# lazy-markdown

Goal: An efficient, safe, modular plain text editor with markdown-aware structure tools, with all dependencies written in Rust (tree-sitter is an exception at the moment, while experimenting with gpui-component).

The basic principle for this project is that minor usability conveniences don't outweigh gross programmatic inefficiencies. That means things like split/live preview probably won't be featured.

The project aims to help people edit markdown source more effectively through structure-aware tools.

## Status

This is currently a GPUI Component plain text tabbed editor. The implementation
strategy is to build a robust plain text editor first, then add markdown-aware
editing features that improve source editing directly.

This program is only tested on Linux/Wayland.

### Current Features

- tabs with per-tab dirty state
- menu actions and hotkeys for new/save/save-as/open
- editor font family picker with per-user persistence
- standard zoom hotkeys for editor font size: `Ctrl+=`, `Ctrl+-`, `Ctrl+0`
- recent documents menu with per-user persistence
- save or discard dialogs on tab/window close
- atomic writing
- dark/light theme options

#### Custom theme

A sample `theme.json` is in the pkg directory.
Add it to ~/.config/lazy-markdown/ and select Custom in the theme menu. The file name must be `theme.json`.

#### Switch to basic editor

The code-editor allows for editor theme settings and markdown syntax highlighting. If you don't care about that you can switch to the basic editor.

Set this in `~/.config/lazy-markdown/config.toml`:

`editor_mode = "basic"`

or

`editor_mode = "code_editor"`


## Possible future enhancements

The current direction and work-in-progress is broadcasted in [issues](https://github.com/Third-Thing/lazy-markdown/issues) / [milestones](https://github.com/Third-Thing/lazy-markdown/milestones).

- hardwrapped line converter
- block quote hotkey
- improved markdown syntax highlighting
- static markdown preview window 
- folder view
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

## Installation

You can use `install.sh` to install locally (without su).

For a system level RPM install on openSUSE:

```sh
cargo install cargo-generate-rpm
cargo build --release
strip -s target/release/lazy-markdown
cargo generate-rpm
zypper install ./target/generate-rpm/<file>.rpm
```

## License

[Blue Oak Model License 1.0.0](https://blueoakcouncil.org/license/1.0.0)

As far as the law allows, this software comes as is,
without any warranty or condition, and no contributor
will be liable to anyone for any damages related to this
software or this license, under any kind of legal claim.
