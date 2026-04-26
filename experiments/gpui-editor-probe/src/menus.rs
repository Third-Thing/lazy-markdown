use gpui::{App, KeyBinding, Menu, MenuItem};
use gpui_component::GlobalState;

use crate::{
    ClearRecentFiles, New, Open, OpenRecent, ResetFontSize, Save, SaveAs, SelectEditorFont, ZoomIn,
    ZoomOut,
    persistence::RecentFiles,
    preferences::{available_editor_fonts, editor_font_label},
};

const MAX_RECENT_LABEL_CHARS: usize = 52;
const MAX_RECENT_NAME_CHARS: usize = 30;

pub(crate) fn install_app_menus(cx: &mut App, recent_files: &RecentFiles, editor_font: &str) {
    cx.bind_keys([
        KeyBinding::new("ctrl-n", New, None),
        KeyBinding::new("ctrl-o", Open, None),
        KeyBinding::new("ctrl-s", Save, None),
        KeyBinding::new("ctrl-shift-s", SaveAs, None),
        KeyBinding::new("ctrl-=", ZoomIn, None),
        KeyBinding::new("ctrl-+", ZoomIn, None),
        KeyBinding::new("ctrl-shift-=", ZoomIn, None),
        KeyBinding::new("ctrl-add", ZoomIn, None),
        KeyBinding::new("ctrl--", ZoomOut, None),
        KeyBinding::new("ctrl-subtract", ZoomOut, None),
        KeyBinding::new("ctrl-0", ResetFontSize, None),
    ]);

    set_app_menus(cx, recent_files, editor_font);
}

pub(crate) fn set_app_menus(cx: &mut App, recent_files: &RecentFiles, editor_font: &str) {
    let owned_menus = build_app_menus(recent_files, editor_font)
        .into_iter()
        .map(Menu::owned)
        .collect();
    cx.set_menus(build_app_menus(recent_files, editor_font));
    GlobalState::global_mut(cx).set_app_menus(owned_menus);
}

fn build_app_menus(recent_files: &RecentFiles, editor_font: &str) -> Vec<Menu> {
    vec![
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New", New),
                MenuItem::separator(),
                MenuItem::action("Open...", Open),
                MenuItem::separator(),
                MenuItem::action("Save", Save),
                MenuItem::action("Save As...", SaveAs),
            ],
            disabled: false,
        },
        Menu {
            name: "Recent".into(),
            items: recent_menu_items(recent_files),
            disabled: false,
        },
        Menu {
            name: "Font".into(),
            items: font_menu_items(editor_font),
            disabled: false,
        },
    ]
}

fn font_menu_items(editor_font: &str) -> Vec<MenuItem> {
    available_editor_fonts()
        .into_iter()
        .map(|font| {
            MenuItem::action(editor_font_label(font), SelectEditorFont(font.to_string()))
                .checked(font == editor_font)
        })
        .collect()
}

fn recent_menu_items(recent_files: &RecentFiles) -> Vec<MenuItem> {
    let mut items: Vec<MenuItem> = recent_files
        .paths()
        .into_iter()
        .map(|path| {
            let label = recent_menu_label(&path);
            MenuItem::action(label, OpenRecent(path.to_string_lossy().into_owned()))
        })
        .collect();

    if items.is_empty() {
        items.push(MenuItem::action("No recent files yet", ClearRecentFiles).disabled(true));
    } else {
        items.push(MenuItem::separator());
        items.push(MenuItem::action("Clear Menu", ClearRecentFiles));
    }

    items
}

fn recent_menu_label(path: &std::path::Path) -> String {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_else(|| "Untitled".into());
    let parent = path
        .parent()
        .map(|parent| parent.display().to_string())
        .unwrap_or_default();
    if parent.is_empty() {
        return shorten_text(&name, MAX_RECENT_LABEL_CHARS);
    }

    let name = shorten_text(&name, MAX_RECENT_NAME_CHARS);
    let parent_budget = MAX_RECENT_LABEL_CHARS.saturating_sub(name.chars().count() + " ()".len());
    let parent = shorten_start(&parent, parent_budget);
    format!("{name} ({parent})")
}

fn shorten_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    format!("{}...", text.chars().take(keep).collect::<String>())
}

fn shorten_start(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let suffix = text
        .chars()
        .rev()
        .take(keep)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("...{suffix}")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{MAX_RECENT_LABEL_CHARS, recent_menu_label, shorten_start};

    #[test]
    fn recent_menu_label_shortens_long_file_names_and_parent_paths() {
        let path = Path::new(
            "/home/example/projects/lazy-markdown/notes/very-long-markdown-document-name-that-would-overflow.md",
        );

        let label = recent_menu_label(path);

        assert!(label.contains("..."));
        assert!(label.chars().count() <= MAX_RECENT_LABEL_CHARS);
    }

    #[test]
    fn shorten_start_keeps_the_end_of_paths_visible() {
        assert_eq!(
            shorten_start("/one/two/three/four/five", 13),
            ".../four/five"
        );
    }
}
