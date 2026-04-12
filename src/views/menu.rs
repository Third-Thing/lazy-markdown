use std::rc::Rc;

use floem::AnyView;
use floem::{
    peniko::Color,
    prelude::{Key, KeyboardEvent, NamedKey, SignalGet, SignalUpdate, *},
    reactive::Effect,
    view::ViewId,
    views::{Container, Empty, Label, Overlay, Stack, dyn_stack},
};

use crate::{
    app_keys::top_level_menu_shortcut,
    commands::{CommandRegistry, command_ids, command_title, invoke_command},
    documents::{current_name, focus_active_document, open_document_path},
    recent_files::clear_recent_files,
    state::{AppState, MenuUiState, TopLevelMenuId},
};

#[derive(Clone)]
struct AppMenuModel {
    id: TopLevelMenuId,
    title: String,
    entries: Vec<AppMenuEntry>,
}

impl AppMenuModel {
    fn new(id: TopLevelMenuId, title: impl Into<String>, entries: Vec<AppMenuEntry>) -> Self {
        Self {
            id,
            title: title.into(),
            entries,
        }
    }

    fn first_selectable_index(&self) -> Option<usize> {
        self.entries.iter().position(AppMenuEntry::is_selectable)
    }

    fn next_selectable_index(&self, current_index: usize, step: isize) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }

        let len = self.entries.len() as isize;
        let mut index = current_index as isize;

        for _ in 0..self.entries.len() {
            index = (index + step).rem_euclid(len);
            if self.entries[index as usize].is_selectable() {
                return Some(index as usize);
            }
        }

        None
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

    fn is_selectable(&self) -> bool {
        matches!(self, Self::Item(item) if item.enabled && item.action.is_some())
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

#[derive(Clone)]
struct PopupRow {
    index: usize,
    entry: AppMenuEntry,
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

fn recent_menu_label(path: &std::path::Path) -> String {
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

fn app_menu_models(command_registry: &CommandRegistry, state: &AppState) -> Vec<AppMenuModel> {
    vec![file_menu_model(command_registry), recent_menu_model(state)]
}

fn menu_model(
    menu_id: TopLevelMenuId,
    command_registry: &CommandRegistry,
    state: &AppState,
) -> AppMenuModel {
    match menu_id {
        TopLevelMenuId::File => file_menu_model(command_registry),
        TopLevelMenuId::Recent => recent_menu_model(state),
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

fn execute_menu_action(state: &AppState, action: Rc<dyn Fn(&AppState)>) {
    close_menu_internal(state, false);
    action(state);
    focus_active_document(state);
}

fn select_menu_index(state: &AppState, menu_id: TopLevelMenuId, selected_index: usize) {
    state.menu_state.set(MenuUiState {
        open_menu: Some(menu_id),
        selected_index,
    });
}

fn popup_row_view(menu_id: TopLevelMenuId, row: PopupRow, state: AppState) -> AnyView {
    match row.entry {
        AppMenuEntry::Separator => Empty::new()
            .style(|s| {
                s.height(1.0)
                    .margin_vert(5.0)
                    .background(Color::from_rgb8(222, 225, 229))
            })
            .into_any(),
        AppMenuEntry::Item(item) => {
            let enabled = item.enabled;
            let action = item.action.clone();
            let click_state = state;
            let hover_state = click_state.clone();

            Label::new(item.title)
                .style(move |s| {
                    let menu_state = click_state.menu_state.get();
                    let selected = menu_state.open_menu == Some(menu_id)
                        && menu_state.selected_index == row.index;
                    s.width_full()
                        .selectable(false)
                        .font_size(14.0)
                        .padding_horiz(12.0)
                        .padding_vert(7.0)
                        .color(if enabled {
                            Color::from_rgb8(44, 50, 63)
                        } else {
                            Color::from_rgb8(148, 154, 166)
                        })
                        .background(if selected && enabled {
                            Color::from_rgb8(210, 215, 222)
                        } else {
                            Color::from_rgb8(235, 237, 240)
                        })
                })
                .on_event_stop(listener::PointerEnter, move |_, _| {
                    if enabled {
                        select_menu_index(&hover_state, menu_id, row.index);
                    }
                })
                .on_event_stop(listener::Click, move |_, _| {
                    let Some(action) = action.clone() else {
                        return;
                    };

                    if enabled {
                        execute_menu_action(&click_state, action);
                    }
                })
                .into_any()
        }
    }
}

fn popup_content_view(
    menu_model: AppMenuModel,
    anchor_id: ViewId,
    command_registry: CommandRegistry,
    state: AppState,
) -> impl IntoView {
    let popup_rows_state = state.clone();
    let popup_style_state = state.clone();
    let popup_row_view_state = state.clone();
    let popup_registry = command_registry.clone();
    let popup_menu_id = menu_model.id;

    Container::derived(move || {
        Stack::vertical_from_iter(
            popup_rows(popup_menu_id, &popup_registry, &popup_rows_state)
                .into_iter()
                .map(|row| popup_row_view(popup_menu_id, row, popup_row_view_state.clone())),
        )
        .style(|s| s.row_gap(2.0).width_full())
    })
    .style(move |s| {
        let is_active = popup_style_state.menu_state.get().open_menu == Some(menu_model.id);
        let anchor_rect = anchor_id.get_visual_rect();
        let inset_left = anchor_rect.x0.round();
        let inset_top = (anchor_rect.y1 + 4.0).round();
        s.fixed()
            .inset_top(inset_top)
            .inset_left(inset_left)
            .min_width(260.0)
            .padding(6.0)
            .border(1.0)
            .border_color(Color::from_rgb8(206, 211, 218))
            .background(Color::from_rgb8(235, 237, 240))
            .z_index(200)
            .apply_if(!is_active, |s| s.hide())
    })
    .on_event_stop(listener::Click, move |_, _| {})
}

fn popup_overlay(
    menu_model: AppMenuModel,
    anchor_id: ViewId,
    popup_id: ViewId,
    command_registry: CommandRegistry,
    state: AppState,
) -> Overlay {
    let popup_child_state = state.clone();
    let popup_key_state = state;
    let popup_child_registry = command_registry.clone();
    let popup_key_registry = command_registry;

    Effect::new({
        let popup_focus_state = popup_child_state.clone();
        move |_| {
            if popup_focus_state.menu_state.get().open_menu == Some(menu_model.id) {
                popup_id.request_focus();
            }
        }
    });

    Overlay::with_id(popup_id)
        .derived_child(move || {
            popup_content_view(
                menu_model.clone(),
                anchor_id,
                popup_child_registry.clone(),
                popup_child_state.clone(),
            )
        })
        .style(|s| s.keyboard_navigable())
        .on_event_stop(listener::KeyDown, {
            move |_, event| {
                handle_open_menu_key_down(&popup_key_state, &popup_key_registry, event);
            }
        })
}

fn menu_button(
    menu_model: AppMenuModel,
    command_registry: CommandRegistry,
    state: AppState,
) -> impl IntoView {
    let anchor_id = ViewId::new();
    let popup_id = ViewId::new();
    state.register_menu_popup(menu_model.id, popup_id);
    popup_id.set_style_parent(anchor_id);
    let button_state = state.clone();
    let button_registry = command_registry.clone();
    let popup = popup_overlay(
        menu_model.clone(),
        anchor_id,
        popup_id,
        command_registry,
        state.clone(),
    );
    let anchor = Container::with_id(
        anchor_id,
        Label::new(menu_model.title.clone())
            .style(move |s| {
                let is_active = state.menu_state.get().open_menu == Some(menu_model.id);
                s.selectable(false)
                    .padding_horiz(6.0)
                    .padding_vert(3.0)
                    .border(1.0)
                    .border_color(if is_active {
                        Color::from_rgb8(162, 170, 182)
                    } else {
                        Color::from_rgb8(196, 199, 204)
                    })
                    .border_radius(4.0)
                    .background(if is_active {
                        Color::from_rgb8(226, 231, 238)
                    } else {
                        Color::from_rgb8(248, 249, 250)
                    })
                    .hover(|s| s.background(Color::from_rgb8(232, 236, 240)))
                    .active(|s| s.background(Color::from_rgb8(218, 224, 230)))
            })
            .on_event_stop(listener::Click, move |_, _| {
                toggle_menu(&button_state, menu_model.id, &button_registry);
            }),
    );

    Stack::new((anchor, popup))
}

pub(crate) fn is_menu_open(state: &AppState) -> bool {
    state.menu_state.get_untracked().open_menu.is_some()
}

pub(crate) fn close_menu(state: &AppState) {
    close_menu_internal(state, true);
}

fn close_menu_internal(state: &AppState, restore_focus: bool) {
    state.menu_state.set(MenuUiState::default());
    if restore_focus {
        focus_active_document(state);
    }
}

pub(crate) fn open_menu(
    state: &AppState,
    menu_id: TopLevelMenuId,
    command_registry: &CommandRegistry,
) {
    let menu_model = menu_model(menu_id, command_registry, state);
    let selected_index = menu_model.first_selectable_index().unwrap_or(0);
    select_menu_index(state, menu_id, selected_index);
    if let Some(popup_id) = state.menu_popup_id(menu_id) {
        popup_id.request_focus();
    }
}

fn toggle_menu(state: &AppState, menu_id: TopLevelMenuId, command_registry: &CommandRegistry) {
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
    let menu_model = menu_model(menu_id, command_registry, state);
    let Some(selected_index) = menu_model.next_selectable_index(menu_state.selected_index, step)
    else {
        return false;
    };

    select_menu_index(state, menu_id, selected_index);
    true
}

fn activate_selected_menu_item(state: &AppState, command_registry: &CommandRegistry) -> bool {
    let menu_state = state.menu_state.get_untracked();
    let Some(menu_id) = menu_state.open_menu else {
        return false;
    };
    let menu_model = menu_model(menu_id, command_registry, state);
    let Some(AppMenuEntry::Item(item)) = menu_model.entries.get(menu_state.selected_index) else {
        return false;
    };
    let Some(action) = item.action.clone() else {
        return false;
    };

    execute_menu_action(state, action);
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

pub(crate) fn menu_bar_view(command_registry: CommandRegistry, state: AppState) -> impl IntoView {
    let menu_state = state.clone();
    let model_registry = command_registry.clone();
    let button_registry = command_registry;
    dyn_stack(
        move || app_menu_models(&model_registry, &menu_state),
        |menu| menu.id,
        move |menu| menu_button(menu, button_registry.clone(), state.clone()),
    )
    .style(|s| s.flex_row().col_gap(6.0))
    .on_event_stop(listener::Click, move |_, _| {})
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
        app_keys::{KeyHandling, app_key_event_config, handle_app_key_down},
        bootstrap::AppBootstrap,
        documents::create_document_state,
        recent_files::RecentFiles,
        state::{AppState, DocumentId, DocumentSet, TopLevelMenuId},
    };

    #[test]
    fn menu_logic_moves_deterministically() {
        let _root = TestRoot::new();
        let bootstrap = AppBootstrap::load().expect("bootstrap");
        let state = test_state(RecentFiles::default());

        super::open_menu(&state, TopLevelMenuId::File, &bootstrap.command_registry);
        assert_eq!(state.menu_state.get_untracked().selected_index, 0);

        assert!(super::handle_open_menu_key_down(
            &state,
            &bootstrap.command_registry,
            &named_key_event(NamedKey::ArrowDown),
        ));
        assert_eq!(state.menu_state.get_untracked().selected_index, 1);

        assert!(super::handle_open_menu_key_down(
            &state,
            &bootstrap.command_registry,
            &named_key_event(NamedKey::ArrowDown),
        ));
        assert_eq!(state.menu_state.get_untracked().selected_index, 3);

        assert!(super::handle_open_menu_key_down(
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

    fn test_state(recent_files: RecentFiles) -> AppState {
        let scope = Scope::new();
        let state = AppState::new(scope, recent_files);
        let initial_document =
            create_document_state(scope, DocumentId::initial(), None, String::from("initial"));
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
            super::menu_bar_view(bootstrap.command_registry.clone(), state.clone()),
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
