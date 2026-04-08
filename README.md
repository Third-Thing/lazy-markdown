lazy-markdown

Currently, it's so lazy that it treats markdown like plain text.

This serves as a basic foundation (plain text notepad)
as well as a useful example for floem.
Although, it currently requires my floem fork's branch
for proper dirty tracking until the PR gets merged.

Features:
- built-in undo/redo
- built-in save/open dialogs
- buttons for new/save/save-as/open
- custom Ctrl+S hotkey
- custom save or discard on close overlay

Buttons are used instead of a menu since 
floem doesn't support them on linux yet.

The save functionality is currently not atomic.

---

License: https://blueoakcouncil.org/license/1.0.0

As far as the law allows, this software comes as is,
without any warranty or condition, and no contributor
will be liable to anyone for any damages related to this
software or this license, under any kind of legal claim.