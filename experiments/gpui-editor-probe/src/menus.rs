use gpui::{App, KeyBinding, Menu, MenuItem};
use gpui_component::GlobalState;

use crate::{New, Open, Save, SaveAs};

pub(crate) fn install_app_menus(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-n", New, None),
        KeyBinding::new("ctrl-o", Open, None),
        KeyBinding::new("ctrl-s", Save, None),
        KeyBinding::new("ctrl-shift-s", SaveAs, None),
    ]);

    let owned_menus = build_app_menus().into_iter().map(Menu::owned).collect();
    cx.set_menus(build_app_menus());
    GlobalState::global_mut(cx).set_app_menus(owned_menus);
}

fn build_app_menus() -> Vec<Menu> {
    vec![Menu {
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
    }]
}
