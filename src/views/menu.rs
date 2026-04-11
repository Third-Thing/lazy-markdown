use std::rc::Rc;

use floem::{
    menu::Menu,
    peniko::Color,
    prelude::*,
    views::{Label, dyn_stack},
};

use crate::{
    commands::{CommandRegistry, command_ids, command_title, invoke_command},
    state::AppState,
};

#[derive(Clone)]
struct AppMenuModel {
    id: &'static str,
    title: String,
    entries: Vec<AppMenuEntry>,
}

impl AppMenuModel {
    fn new(id: &'static str, title: impl Into<String>, entries: Vec<AppMenuEntry>) -> Self {
        Self {
            id,
            title: title.into(),
            entries,
        }
    }
}

#[derive(Clone)]
enum AppMenuEntry {
    Separator,
    Item(AppMenuItem),
}

impl AppMenuEntry {
    fn item(title: impl Into<String>, action: impl Fn(&AppState) + 'static) -> Self {
        Self::Item(AppMenuItem::new(title, action))
    }

    fn disabled(title: impl Into<String>) -> Self {
        Self::Item(AppMenuItem::disabled(title))
    }
}

#[derive(Clone)]
struct AppMenuItem {
    title: String,
    enabled: bool,
    action: Option<Rc<dyn Fn(&AppState)>>,
}

impl AppMenuItem {
    fn new(title: impl Into<String>, action: impl Fn(&AppState) + 'static) -> Self {
        Self {
            title: title.into(),
            enabled: true,
            action: Some(Rc::new(action)),
        }
    }

    fn disabled(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            enabled: false,
            action: None,
        }
    }
}

fn command_menu_entry(
    command_registry: &CommandRegistry,
    command_id: &'static str,
) -> AppMenuEntry {
    let title = command_title(command_registry, command_id).to_string();
    AppMenuEntry::item(title, move |state| invoke_command(command_id, state))
}

fn file_menu_model(command_registry: &CommandRegistry) -> AppMenuModel {
    AppMenuModel::new(
        "file",
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

fn recent_menu_model() -> AppMenuModel {
    AppMenuModel::new(
        "recent",
        "Recent",
        vec![AppMenuEntry::disabled("No recent files yet")],
    )
}

fn app_menu_models(command_registry: &CommandRegistry) -> Vec<AppMenuModel> {
    vec![file_menu_model(command_registry), recent_menu_model()]
}

fn build_menu_entry(menu: Menu, entry: &AppMenuEntry, state: AppState) -> Menu {
    match entry {
        AppMenuEntry::Separator => menu.separator(),
        AppMenuEntry::Item(item) => {
            let title = item.title.clone();
            let enabled = item.enabled;
            let action = item.action.clone();
            menu.item(title, move |menu_item| {
                let menu_item = menu_item.enabled(enabled);
                let Some(action) = action.clone() else {
                    return menu_item;
                };
                let click_state = state.clone();
                menu_item.action(move || action(&click_state))
            })
        }
    }
}

fn build_menu_from_model(menu_model: &AppMenuModel, state: AppState) -> Menu {
    let mut menu = Menu::new();
    for entry in &menu_model.entries {
        menu = build_menu_entry(menu, entry, state.clone());
    }
    menu
}

fn menu_button(menu_model: AppMenuModel, state: AppState) -> impl IntoView {
    Label::new(menu_model.title.clone())
        .style(|s| {
            s.selectable(false)
                .padding_horiz(10.0)
                .padding_vert(6.0)
                .border(1.0)
                .border_color(Color::from_rgb8(196, 199, 204))
                .border_radius(6.0)
                .background(Color::from_rgb8(248, 249, 250))
                .hover(|s| s.background(Color::from_rgb8(232, 236, 240)))
                .active(|s| s.background(Color::from_rgb8(218, 224, 230)))
        })
        .popout_menu(move || build_menu_from_model(&menu_model, state.clone()))
}

pub(crate) fn menu_bar_view(command_registry: CommandRegistry, state: AppState) -> impl IntoView {
    dyn_stack(
        move || app_menu_models(&command_registry),
        |menu| menu.id,
        move |menu| menu_button(menu, state.clone()),
    )
    .style(|s| s.flex_row().col_gap(6.0))
}
