use floem::{
    context::{EventCallbackConfig, Phases},
    prelude::{Key, KeyboardEvent, Modifiers, SignalGet},
};

use crate::{
    commands::{CommandRegistry, invoke_command},
    shortcuts::resolve_shortcut_command,
    state::{AppState, TopLevelMenuId},
    views::menu::{close_menu, handle_open_menu_key_down, is_menu_open, open_menu},
};

pub(crate) enum KeyHandling {
    Handled,
    NotHandled,
}

pub(crate) fn app_key_event_config() -> EventCallbackConfig {
    EventCallbackConfig {
        phases: Phases::CAPTURE | Phases::TARGET | Phases::BROADCAST,
    }
}

pub(crate) fn handle_app_key_down(
    state: &AppState,
    command_registry: &CommandRegistry,
    event: &KeyboardEvent,
) -> KeyHandling {
    if let Some(menu_id) = top_level_menu_shortcut(event) {
        if state.menu_state.get_untracked().open_menu == Some(menu_id) {
            close_menu(state);
        } else {
            open_menu(state, menu_id, command_registry);
        }
        return KeyHandling::Handled;
    }

    if is_menu_open(state) {
        handle_open_menu_key_down(state, command_registry, event);
        return KeyHandling::Handled;
    }

    if let Some(command_id) = resolve_shortcut_command(command_registry, event) {
        invoke_command(command_id, state);
        return KeyHandling::Handled;
    }

    KeyHandling::NotHandled
}

pub(crate) fn top_level_menu_shortcut(event: &KeyboardEvent) -> Option<TopLevelMenuId> {
    if event.modifiers != Modifiers::ALT {
        return None;
    }

    match &event.key {
        Key::Character(key) if key.eq_ignore_ascii_case("f") => Some(TopLevelMenuId::File),
        Key::Character(key) if key.eq_ignore_ascii_case("r") => Some(TopLevelMenuId::Recent),
        Key::Character(key) if key.eq_ignore_ascii_case("t") => Some(TopLevelMenuId::Theme),
        _ => None,
    }
}
