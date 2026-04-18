use std::path::PathBuf;

mod bootstrap;
mod commands;
mod dialogs;
mod menus;
mod persistence;
mod preferences;
mod shortcuts;
mod workspace;

use bootstrap::AppBootstrap;
use dialogs::{ActiveDialog, dialog_overlay};
use floem::{
    Application,
    action::current_theme,
    event::EventPropagation,
    prelude::*,
    reactive::Scope,
    views::Stack,
    window::{Theme as WindowTheme, WindowConfig, WindowId},
};
use menus::{
    KeyHandling, app_key_event_config, close_menu, handle_app_key_down, is_menu_open, menu_bar_view,
};
use persistence::recent_files::{RecentFiles, load_recent_files, record_recent_file};
use preferences::theme::{self, sync_theme_preference};
use workspace::{
    AppState, DocumentId, DocumentSet, activate_document, create_document_state, current_name,
    document_title_text, workspace_frame_view,
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

    Stack::new((
        workspace_frame_view(menu_bar, state.clone()),
        dialog_overlay(state.clone()),
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
                state.set_active_dialog(ActiveDialog::ConfirmCloseWindow {
                    window_id,
                    remaining_documents: dirty_documents,
                });
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
