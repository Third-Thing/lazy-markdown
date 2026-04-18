mod keys;
mod model;
mod view;

use std::path::Path;

use floem::prelude::{SignalGet, SignalUpdate};

pub(crate) use keys::{
    KeyHandling, app_key_event_config, close_menu, handle_app_key_down, handle_open_menu_key_down,
    is_menu_open,
};
use model::{AppMenuEntry, AppMenuModel, PopupRow};
pub(crate) use view::menu_bar_view;

use crate::{
    commands::{CommandRegistry, command_ids, command_title, run_command},
    persistence::recent_files::clear_recent_files,
    preferences::{
        editor_font::{apply_editor_font, available_editor_fonts, editor_font_label},
        theme::{ThemePreference, apply_theme_preference},
    },
    workspace::{AppState, TopLevelMenuId, current_name, open_document_path},
};

fn close_menu_internal(state: &AppState, restore_focus: bool) {
    state
        .menu_state
        .set(crate::workspace::MenuUiState::default());
    if restore_focus {
        crate::workspace::focus_active_document(state);
    }
}

fn execute_menu_action(state: &AppState, action: std::rc::Rc<dyn Fn(&AppState)>) {
    close_menu_internal(state, false);
    action(state);
    crate::workspace::focus_active_document(state);
}

fn select_menu_index(state: &AppState, menu_id: TopLevelMenuId, selected_index: usize) {
    state.menu_state.set(crate::workspace::MenuUiState {
        open_menu: Some(menu_id),
        selected_index,
    });
}

fn command_menu_entry(
    command_registry: &CommandRegistry,
    command_id: &'static str,
) -> AppMenuEntry {
    let title = command_title(command_registry, command_id).to_string();
    AppMenuEntry::item(title, move |state| run_command(command_id, state))
}

fn file_menu_model(command_registry: &CommandRegistry) -> AppMenuModel {
    AppMenuModel::new(
        TopLevelMenuId::File,
        "File",
        vec![
            command_menu_entry(command_registry, command_ids::FILE_NEW),
            command_menu_entry(command_registry, command_ids::FILE_OPEN),
            AppMenuEntry::Separator,
            command_menu_entry(command_registry, command_ids::FILE_SAVE),
            command_menu_entry(command_registry, command_ids::FILE_SAVE_AS),
        ],
    )
}

fn recent_menu_label(path: &Path) -> String {
    let name = current_name(Some(path));
    format!("{name} ({})", path.display())
}

fn recent_menu_model(state: &AppState) -> AppMenuModel {
    let recent_paths = state.recent_files.get().paths();
    let mut entries: Vec<AppMenuEntry> = recent_paths
        .into_iter()
        .map(|path| {
            let title = recent_menu_label(&path);
            AppMenuEntry::item(title, move |state| open_document_path(state, path.clone()))
        })
        .collect();

    if entries.is_empty() {
        entries.push(AppMenuEntry::disabled("No recent files yet"));
    } else {
        entries.push(AppMenuEntry::Separator);
        entries.push(AppMenuEntry::item("Clear Menu", clear_recent_files));
    }

    AppMenuModel::new(TopLevelMenuId::Recent, "Recent", entries)
}

fn theme_menu_entry(
    state: &AppState,
    preference: ThemePreference,
    title: &'static str,
) -> AppMenuEntry {
    let marker = if state.theme_preference() == preference {
        "✓"
    } else {
        "•"
    };
    let title = format!("{marker} {title}");
    AppMenuEntry::item(title, move |state| {
        apply_theme_preference(state, preference);
    })
}

fn theme_menu_model(state: &AppState) -> AppMenuModel {
    AppMenuModel::new(
        TopLevelMenuId::Theme,
        "Theme",
        vec![
            theme_menu_entry(state, ThemePreference::Light, "Light"),
            theme_menu_entry(state, ThemePreference::Dark, "Dark"),
            theme_menu_entry(state, ThemePreference::FollowOs, "Follow OS"),
        ],
    )
}

fn font_menu_entry(state: &AppState, font_family: String) -> AppMenuEntry {
    let marker = if state.editor_font() == font_family {
        "✓"
    } else {
        "•"
    };
    let title = format!("{marker} {}", editor_font_label(&font_family));
    AppMenuEntry::item(title, move |state| {
        apply_editor_font(state, font_family.clone());
    })
}

fn font_menu_model(state: &AppState) -> AppMenuModel {
    let entries = available_editor_fonts()
        .into_iter()
        .map(|font_family| font_menu_entry(state, font_family))
        .collect();

    AppMenuModel::new(TopLevelMenuId::Font, "Font", entries)
}

fn app_menu_models(command_registry: &CommandRegistry, state: &AppState) -> Vec<AppMenuModel> {
    vec![
        file_menu_model(command_registry),
        recent_menu_model(state),
        theme_menu_model(state),
        font_menu_model(state),
    ]
}

fn menu_model(
    menu_id: TopLevelMenuId,
    command_registry: &CommandRegistry,
    state: &AppState,
) -> AppMenuModel {
    match menu_id {
        TopLevelMenuId::File => file_menu_model(command_registry),
        TopLevelMenuId::Recent => recent_menu_model(state),
        TopLevelMenuId::Theme => theme_menu_model(state),
        TopLevelMenuId::Font => font_menu_model(state),
    }
}

fn popup_rows(
    menu_id: TopLevelMenuId,
    command_registry: &CommandRegistry,
    state: &AppState,
) -> Vec<PopupRow> {
    menu_model(menu_id, command_registry, state)
        .entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| PopupRow { index, entry })
        .collect()
}
