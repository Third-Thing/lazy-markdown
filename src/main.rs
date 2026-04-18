use std::path::PathBuf;

mod app_keys;
mod bootstrap;
mod commands;
mod documents;
mod persistence;
mod preferences;
mod shortcuts;
mod state;
mod views;

use app_keys::{KeyHandling, app_key_event_config, handle_app_key_down};
use bootstrap::AppBootstrap;
use documents::{activate_document, create_document_state, current_name, document_title_text};
use floem::{
    Application,
    action::current_theme,
    event::EventPropagation,
    prelude::*,
    reactive::Scope,
    views::{Label, Stack},
    window::{Theme as WindowTheme, WindowConfig, WindowId},
};
use preferences::{editor_font::editor_font_label, theme::{self, sync_theme_preference}};
use persistence::recent_files::{RecentFiles, load_recent_files, record_recent_file};
use state::{AppState, DocumentId, DocumentSet, PendingAction};
use views::menu::{close_menu, is_menu_open};
use views::{
    dialogs::confirm_overlay, editor::tab_content_view, menu::menu_bar_view, tabs::tab_strip_view,
};

fn app_view(window_id: WindowId, bootstrap: AppBootstrap) -> impl IntoView {
    let (recent_files, recent_files_error) = match load_recent_files() {
        Ok(recent_files) => (recent_files, None),
        Err(err) => (RecentFiles::default(), Some(err)),
    };
    let state = AppState::new(Scope::current(), recent_files, bootstrap.app_config.clone());
    state
        .window_theme
        .set(current_theme().unwrap_or(WindowTheme::Light));
    sync_theme_preference(&state);

    if let Some(err) = recent_files_error {
        state.status_message.set(Some(err));
    }
    if let Some(err) = bootstrap.app_config_error.clone() {
        state.status_message.set(Some(err));
    }

    let mut initial_path = std::env::args().nth(1).map(PathBuf::from);
    let initial_text = match initial_path.as_ref() {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                state
                    .status_message
                    .set(Some(format!("Failed to open {}: {err}", path.display())));
                initial_path = None;
                String::new()
            }
        },
        None => String::new(),
    };

    let initial_document = create_document_state(
        state.document_scope,
        DocumentId::initial(),
        initial_path,
        initial_text,
        state.editor_font_untracked(),
        state.editor_font_size_untracked(),
    );
    state.documents.set(DocumentSet::new(initial_document));
    if let Some(path) = state
        .active_document_untracked()
        .and_then(|document| document.file_path.get_untracked())
    {
        record_recent_file(&state, &path);
    }

    let menu_bar = menu_bar_view(bootstrap.command_registry.clone(), state.clone());

    let top_bar = Stack::horizontal((menu_bar,)).style({
        let state = state.clone();
        move |s| {
            let theme = state.app_theme();
            s.width_full()
                .items_center()
                .padding_horiz(10.0)
                .padding_vert(6.0)
                .background(theme.chrome_bg)
        }
    });

    let status_strip = Stack::horizontal((
        {
            let state = state.clone();
            Label::derived(move || {
                format!(
                    "{} {}px",
                    editor_font_label(&state.editor_font()),
                    state.editor_font_size()
                )
            })
        }
        .style({
            let state = state.clone();
            move |s| {
                let theme = state.app_theme();
                s.font_size(12.0).font_bold().color(theme.text)
            }
        }),
        {
            let state = state.clone();
            Label::derived(move || state.status_message.get().unwrap_or_default())
        }
        .style({
            let state = state.clone();
            move |s| {
                let theme = state.app_theme();
                s.font_size(12.0)
                    .color(theme.text_muted)
                    .text_ellipsis()
                    .min_width(0.0)
                    .flex_grow(1.0)
                    .justify_end()
            }
        }),
    ))
    .style({
        let state = state.clone();
        move |s| {
            let theme = state.app_theme();
            s.width_full()
                .items_center()
                .justify_between()
                .col_gap(12.0)
                .padding_horiz(12.0)
                .padding_vert(8.0)
                .background(theme.status_bg)
                .border_top(1.0)
                .border_color(theme.border)
        }
    });

    let tabs_strip = tab_strip_view(state.clone());
    let tabs_content = tab_content_view(state.clone());

    Stack::new((
        Stack::vertical((top_bar, tabs_strip, tabs_content, status_strip)).style({
            let state = state.clone();
            move |s| {
                let theme = state.app_theme();
                s.size_full()
                    .padding(10.0)
                    .row_gap(0.0)
                    .background(theme.panel_bg)
            }
        }),
        confirm_overlay(state.clone()),
    ))
    .style(|s| s.size_full())
    .window_title({
        let state = state.clone();
        move || {
            let Some(document) = state.active_document() else {
                return current_name(None);
            };
            document_title_text(&document)
        }
    })
    .on_event_with_config(listener::KeyDown, app_key_event_config(), {
        let state = state.clone();
        let command_registry = bootstrap.command_registry.clone();
        move |cx, event| match handle_app_key_down(&state, &command_registry, event) {
            KeyHandling::Handled => {
                cx.prevent_default();
                EventPropagation::Stop
            }
            KeyHandling::NotHandled => EventPropagation::Continue,
        }
    })
    .on_event_cont(listener::Click, {
        let state = state.clone();
        move |_, _| {
            if is_menu_open(&state) {
                close_menu(&state);
            }
        }
    })
    .on_event_cont(listener::WindowCloseRequested, {
        let state = state.clone();
        move |cx, _| {
            let dirty_documents = state.dirty_document_ids_untracked();
            if let Some(first_dirty_document) = dirty_documents.first().copied() {
                cx.prevent_default();
                activate_document(&state, first_dirty_document);
                state.pending_action.set(Some(PendingAction::CloseWindow {
                    window_id,
                    remaining_documents: dirty_documents,
                }));
                state.show_confirm.set(true);
                return;
            }

            if let Err(err) = state.store_app_config() {
                eprintln!("Failed to save settings on exit: {err}");
            }
        }
    })
    .on_event_cont(listener::ThemeChanged, {
        let state = state.clone();
        move |_, theme| {
            if state.theme_preference_untracked() == theme::ThemePreference::FollowOs {
                state.window_theme.set(*theme);
            }
        }
    })
}

fn main() {
    let bootstrap = match AppBootstrap::load() {
        Ok(bootstrap) => bootstrap,
        Err(err) => {
            eprintln!("Failed to start lazy-markdown: {err}");
            return;
        }
    };

    Application::new()
        .window(
            move |window_id| app_view(window_id, bootstrap.clone()),
            Some(
                WindowConfig::default()
                    .size((920.0, 680.0))
                    .min_size((480.0, 320.0))
                    .title("lazy-markdown"),
            ),
        )
        .run();
}
