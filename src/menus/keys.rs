use floem::{
    context::{EventCallbackConfig, Phases},
    prelude::{Key, KeyboardEvent, Modifiers, NamedKey, SignalGet},
};

use crate::{
    commands::CommandRegistry,
    menus::model::AppMenuEntry,
    shortcuts::resolve_shortcut_command,
    workspace::{AppState, TopLevelMenuId},
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
        crate::commands::run_command(command_id, state);
        return KeyHandling::Handled;
    }

    KeyHandling::NotHandled
}

pub(crate) fn is_menu_open(state: &AppState) -> bool {
    state.menu_state.get_untracked().open_menu.is_some()
}

pub(crate) fn close_menu(state: &AppState) {
    crate::menus::close_menu_internal(state, true);
}

pub(crate) fn open_menu(
    state: &AppState,
    menu_id: TopLevelMenuId,
    command_registry: &CommandRegistry,
) {
    let menu_model = crate::menus::menu_model(menu_id, command_registry, state);
    let selected_index = menu_model.first_selectable_index().unwrap_or(0);
    crate::menus::select_menu_index(state, menu_id, selected_index);
    if let Some(popup_id) = state.menu_popup_id(menu_id) {
        popup_id.request_focus();
    }
}

pub(crate) fn toggle_menu(
    state: &AppState,
    menu_id: TopLevelMenuId,
    command_registry: &CommandRegistry,
) {
    if state.menu_state.get_untracked().open_menu == Some(menu_id) {
        close_menu(state);
    } else {
        open_menu(state, menu_id, command_registry);
    }
}

fn move_menu_selection(
    state: &AppState,
    command_registry: &CommandRegistry,
    menu_id: TopLevelMenuId,
    step: isize,
) -> bool {
    let menu_state = state.menu_state.get_untracked();
    let menu_model = crate::menus::menu_model(menu_id, command_registry, state);
    let Some(selected_index) = menu_model.next_selectable_index(menu_state.selected_index, step)
    else {
        return false;
    };

    crate::menus::select_menu_index(state, menu_id, selected_index);
    true
}

fn activate_selected_menu_item(state: &AppState, command_registry: &CommandRegistry) -> bool {
    let menu_state = state.menu_state.get_untracked();
    let Some(menu_id) = menu_state.open_menu else {
        return false;
    };
    let menu_model = crate::menus::menu_model(menu_id, command_registry, state);
    let Some(AppMenuEntry::Item(item)) = menu_model.entries.get(menu_state.selected_index) else {
        return false;
    };
    let Some(action) = item.action.clone() else {
        return false;
    };

    crate::menus::execute_menu_action(state, action);
    true
}

pub(crate) fn handle_open_menu_key_down(
    state: &AppState,
    command_registry: &CommandRegistry,
    event: &KeyboardEvent,
) -> bool {
    let Some(active_menu) = state.menu_state.get_untracked().open_menu else {
        return false;
    };

    if let Some(menu_id) = top_level_menu_shortcut(event) {
        if menu_id == active_menu {
            close_menu(state);
        } else {
            open_menu(state, menu_id, command_registry);
        }
        return true;
    }

    match event.key {
        Key::Named(NamedKey::Escape) => {
            close_menu(state);
            true
        }
        Key::Named(NamedKey::ArrowDown) => {
            move_menu_selection(state, command_registry, active_menu, 1);
            true
        }
        Key::Named(NamedKey::ArrowUp) => {
            move_menu_selection(state, command_registry, active_menu, -1);
            true
        }
        Key::Named(NamedKey::ArrowRight) => {
            open_menu(state, active_menu.next(), command_registry);
            true
        }
        Key::Named(NamedKey::ArrowLeft) => {
            open_menu(state, active_menu.previous(), command_registry);
            true
        }
        Key::Named(NamedKey::Enter) => {
            activate_selected_menu_item(state, command_registry);
            true
        }
        _ => false,
    }
}

fn top_level_menu_shortcut(event: &KeyboardEvent) -> Option<TopLevelMenuId> {
    if event.modifiers != Modifiers::ALT {
        return None;
    }

    match &event.key {
        Key::Character(key) if key.eq_ignore_ascii_case("f") => Some(TopLevelMenuId::File),
        Key::Character(key) if key.eq_ignore_ascii_case("r") => Some(TopLevelMenuId::Recent),
        Key::Character(key) if key.eq_ignore_ascii_case("t") => Some(TopLevelMenuId::Theme),
        Key::Character(key) if key.eq_ignore_ascii_case("o") => Some(TopLevelMenuId::Font),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use floem::{
        context::EventCx,
        event::{Event, EventPropagation},
        headless::{HeadlessHarness, TestRoot},
        prelude::{
            Code, Decorators, Key, KeyState, KeyboardEvent, Modifiers, NamedKey, SignalGet,
            SignalUpdate, listener,
        },
        reactive::Scope,
        view::ViewId,
        views::{Container, Label, Stack},
    };

    use crate::{
        bootstrap::AppBootstrap,
        menus::{KeyHandling, app_key_event_config, handle_app_key_down, menu_bar_view},
        persistence::{config::AppConfig, recent_files::RecentFiles},
        preferences::editor_font::{MONOSPACE_FONT, default_editor_font_size},
        workspace::{AppState, DocumentId, DocumentSet, TopLevelMenuId, create_document_state},
    };

    use super::{handle_open_menu_key_down, open_menu};

    #[test]
    fn menu_logic_moves_deterministically() {
        let _root = TestRoot::new();
        let bootstrap = AppBootstrap::load().expect("bootstrap");
        let state = test_state(RecentFiles::default());

        open_menu(&state, TopLevelMenuId::File, &bootstrap.command_registry);
        assert_eq!(state.menu_state.get_untracked().selected_index, 0);

        assert!(handle_open_menu_key_down(
            &state,
            &bootstrap.command_registry,
            &named_key_event(NamedKey::ArrowDown),
        ));
        assert_eq!(state.menu_state.get_untracked().selected_index, 1);

        assert!(handle_open_menu_key_down(
            &state,
            &bootstrap.command_registry,
            &named_key_event(NamedKey::ArrowDown),
        ));
        assert_eq!(state.menu_state.get_untracked().selected_index, 3);

        assert!(handle_open_menu_key_down(
            &state,
            &bootstrap.command_registry,
            &named_key_event(NamedKey::ArrowUp),
        ));
        assert_eq!(state.menu_state.get_untracked().selected_index, 1);
    }

    #[test]
    fn shell_key_events_select_and_open_expected_recent_file() {
        let root = TestRoot::new();
        let bootstrap = AppBootstrap::load().expect("bootstrap");
        let recent_a = temp_markdown_file("menu-recent-a", "a");
        let recent_b = temp_markdown_file("menu-recent-b", "b");
        let recent_files = RecentFiles::from_paths(vec![recent_a.clone(), recent_b.clone()]);
        let state = test_state(recent_files);
        let focus_id = ViewId::new();
        let mut harness = menu_harness(root, state.clone(), bootstrap.clone(), focus_id);

        harness.rebuild();
        focus_id.request_focus();
        harness.process_update_no_paint();
        assert!(harness.is_focused(focus_id));

        dispatch_key_and_flush(&mut harness, alt_character_event("r"));
        assert_eq!(
            state.menu_state.get_untracked().open_menu,
            Some(TopLevelMenuId::Recent)
        );
        assert_eq!(state.menu_state.get_untracked().selected_index, 0);
        let popup_id = state
            .menu_popup_id(TopLevelMenuId::Recent)
            .expect("recent popup id");
        assert!(harness.is_focused(popup_id));

        dispatch_key_and_flush(&mut harness, named_key_event(NamedKey::ArrowDown));
        assert_eq!(state.menu_state.get_untracked().selected_index, 1);

        dispatch_key_and_flush(&mut harness, named_key_event(NamedKey::Enter));
        let active_path = state
            .active_document_untracked()
            .and_then(|document| document.file_path.get_untracked());
        assert_eq!(active_path.as_ref(), Some(&recent_b));

        let _ = fs::remove_file(recent_a);
        let _ = fs::remove_file(recent_b);
    }

    #[test]
    fn shell_capture_beats_focused_child_arrow_handler_when_menu_is_open() {
        let root = TestRoot::new();
        let bootstrap = AppBootstrap::load().expect("bootstrap");
        let state = test_state(RecentFiles::default());
        let focus_id = ViewId::new();
        let child_keydowns = Scope::new().create_rw_signal(0usize);
        let body = Container::with_id(
            focus_id,
            Label::new("body")
                .style(|s| s.padding(20.0))
                .on_event_stop(listener::KeyDown, {
                    let child_keydowns = child_keydowns;
                    move |_, _| {
                        child_keydowns.update(|count| *count += 1);
                    }
                }),
        )
        .style(|s| s.width_full().height_full().keyboard_navigable());
        let mut harness = menu_harness_with_body(root, state.clone(), bootstrap.clone(), body);

        harness.rebuild();
        focus_id.request_focus();
        harness.process_update_no_paint();
        assert!(harness.is_focused(focus_id));

        dispatch_key_and_flush(&mut harness, alt_character_event("f"));
        assert_eq!(
            state.menu_state.get_untracked().open_menu,
            Some(TopLevelMenuId::File)
        );
        assert_eq!(state.menu_state.get_untracked().selected_index, 0);
        let popup_id = state
            .menu_popup_id(TopLevelMenuId::File)
            .expect("file popup id");
        assert!(harness.is_focused(popup_id));

        dispatch_key_and_flush(&mut harness, named_key_event(NamedKey::ArrowDown));
        assert_eq!(state.menu_state.get_untracked().selected_index, 1);
        assert_eq!(child_keydowns.get_untracked(), 0);
    }

    #[test]
    fn shell_zoom_shortcuts_change_editor_font_size() {
        let root = TestRoot::new();
        let bootstrap = AppBootstrap::load().expect("bootstrap");
        let state = test_state(RecentFiles::default());
        let focus_id = ViewId::new();
        let mut harness = menu_harness(root, state.clone(), bootstrap, focus_id);

        harness.rebuild();
        focus_id.request_focus();
        harness.process_update_no_paint();
        assert!(harness.is_focused(focus_id));

        dispatch_key_and_flush(
            &mut harness,
            control_shift_character_event("+", Code::Equal),
        );
        assert_eq!(
            state.editor_font_size_untracked(),
            default_editor_font_size() + 1
        );

        dispatch_key_and_flush(&mut harness, control_character_event("-", Code::Minus));
        assert_eq!(
            state.editor_font_size_untracked(),
            default_editor_font_size()
        );

        dispatch_key_and_flush(&mut harness, control_character_event("+", Code::NumpadAdd));
        assert_eq!(
            state.editor_font_size_untracked(),
            default_editor_font_size() + 1
        );

        dispatch_key_and_flush(&mut harness, control_character_event("0", Code::Digit0));
        assert_eq!(
            state.editor_font_size_untracked(),
            default_editor_font_size()
        );
    }

    #[test]
    fn font_menu_can_be_opened_and_apply_a_font() {
        let root = TestRoot::new();
        let bootstrap = AppBootstrap::load().expect("bootstrap");
        let state = test_state(RecentFiles::default());
        let focus_id = ViewId::new();
        let mut harness = menu_harness(root, state.clone(), bootstrap.clone(), focus_id);

        harness.rebuild();
        focus_id.request_focus();
        harness.process_update_no_paint();
        assert!(harness.is_focused(focus_id));

        dispatch_key_and_flush(&mut harness, alt_character_event("o"));
        assert_eq!(
            state.menu_state.get_untracked().open_menu,
            Some(TopLevelMenuId::Font)
        );
        assert_eq!(state.menu_state.get_untracked().selected_index, 0);
        let popup_id = state
            .menu_popup_id(TopLevelMenuId::Font)
            .expect("font popup id");
        assert!(harness.is_focused(popup_id));

        dispatch_key_and_flush(&mut harness, named_key_event(NamedKey::ArrowDown));
        dispatch_key_and_flush(&mut harness, named_key_event(NamedKey::ArrowDown));
        dispatch_key_and_flush(&mut harness, named_key_event(NamedKey::ArrowDown));
        dispatch_key_and_flush(&mut harness, named_key_event(NamedKey::Enter));

        assert_eq!(state.editor_font_untracked(), MONOSPACE_FONT);
    }

    fn test_state(recent_files: RecentFiles) -> AppState {
        let scope = Scope::new();
        let state = AppState::new(scope, recent_files, AppConfig::default());
        let initial_document = create_document_state(
            scope,
            DocumentId::initial(),
            None,
            String::from("initial"),
            state.editor_font_untracked(),
            state.editor_font_size_untracked(),
        );
        state.documents.set(DocumentSet::new(initial_document));
        state
    }

    fn menu_harness(
        root: TestRoot,
        state: AppState,
        bootstrap: AppBootstrap,
        focus_id: ViewId,
    ) -> HeadlessHarness {
        menu_harness_with_body(
            root,
            state,
            bootstrap,
            Container::with_id(focus_id, Label::new("body").style(|s| s.padding(20.0)))
                .style(|s| s.width_full().height_full().keyboard_navigable()),
        )
    }

    fn menu_harness_with_body(
        root: TestRoot,
        state: AppState,
        bootstrap: AppBootstrap,
        body: impl floem::IntoView + 'static,
    ) -> HeadlessHarness {
        let view = Stack::vertical((
            menu_bar_view(bootstrap.command_registry.clone(), state.clone()),
            body,
        ))
        .style(|s| s.size_full())
        .on_event_with_config(listener::KeyDown, app_key_event_config(), {
            let state = state.clone();
            let command_registry = bootstrap.command_registry.clone();
            move |cx: &mut EventCx<'_>, event| match handle_app_key_down(
                &state,
                &command_registry,
                event,
            ) {
                KeyHandling::Handled => {
                    cx.prevent_default();
                    EventPropagation::Stop
                }
                KeyHandling::NotHandled => EventPropagation::Continue,
            }
        });

        HeadlessHarness::new_with_size(root, view, 920.0, 680.0)
    }

    fn dispatch_key(harness: &mut HeadlessHarness, key_event: KeyboardEvent) {
        harness.dispatch_event(Event::Key(key_event));
    }

    fn dispatch_key_and_flush(harness: &mut HeadlessHarness, key_event: KeyboardEvent) {
        dispatch_key(harness, key_event);
        harness.process_update_no_paint();
    }

    fn named_key_event(key: NamedKey) -> KeyboardEvent {
        let code = match key {
            NamedKey::ArrowDown => Code::ArrowDown,
            NamedKey::ArrowUp => Code::ArrowUp,
            NamedKey::ArrowLeft => Code::ArrowLeft,
            NamedKey::ArrowRight => Code::ArrowRight,
            NamedKey::Enter => Code::Enter,
            NamedKey::Escape => Code::Escape,
            _ => panic!("unsupported named key in test"),
        };
        KeyboardEvent {
            state: KeyState::Down,
            key: Key::Named(key),
            code,
            ..Default::default()
        }
    }

    fn alt_character_event(key: &'static str) -> KeyboardEvent {
        let code = match key {
            "f" | "F" => Code::KeyF,
            "r" | "R" => Code::KeyR,
            "o" | "O" => Code::KeyO,
            _ => panic!("unsupported character key in test"),
        };
        KeyboardEvent {
            state: KeyState::Down,
            key: Key::Character(key.into()),
            code,
            modifiers: Modifiers::ALT,
            ..Default::default()
        }
    }

    fn control_character_event(key: &'static str, code: Code) -> KeyboardEvent {
        KeyboardEvent {
            state: KeyState::Down,
            key: Key::Character(key.into()),
            code,
            modifiers: Modifiers::CONTROL,
            ..Default::default()
        }
    }

    fn control_shift_character_event(key: &'static str, code: Code) -> KeyboardEvent {
        KeyboardEvent {
            state: KeyState::Down,
            key: Key::Character(key.into()),
            code,
            modifiers: Modifiers::CONTROL | Modifiers::SHIFT,
            ..Default::default()
        }
    }

    fn temp_markdown_file(prefix: &str, contents: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{unique}.md"));
        fs::write(&path, contents).expect("write temp file");
        path
    }
}
