use std::path::PathBuf;

mod bootstrap;
mod commands;
mod documents;
mod recent_files;
mod shortcuts;
mod state;
mod theme;
mod views;

use bootstrap::AppBootstrap;
use documents::{activate_document, create_document_state, current_name, document_title_text};
use floem::{
    Application,
    peniko::Color,
    prelude::*,
    reactive::Scope,
    views::{Label, Stack},
    window::{WindowConfig, WindowId},
};
use recent_files::{RecentFiles, load_recent_files, record_recent_file};
use state::{AppState, DocumentId, DocumentSet, PendingAction};
use views::{
    dialogs::confirm_overlay, editor::tab_content_view, menu::menu_bar_view, tabs::tab_strip_view,
};

fn app_view(window_id: WindowId, bootstrap: AppBootstrap) -> impl IntoView {
    let (recent_files, recent_files_error) = match load_recent_files() {
        Ok(recent_files) => (recent_files, None),
        Err(err) => (RecentFiles::default(), Some(err)),
    };
    let state = AppState::new(Scope::current(), recent_files);

    if let Some(err) = recent_files_error {
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
    );
    state.documents.set(DocumentSet::new(initial_document));
    if let Some(path) = state
        .active_document_untracked()
        .and_then(|document| document.file_path.get_untracked())
    {
        record_recent_file(&state, &path);
    }

    let menu_bar = menu_bar_view(bootstrap.command_registry.clone(), state.clone());

    let top_bar = {
        let state = state.clone();
        Stack::horizontal((
            menu_bar,
            Label::derived(move || {
                let Some(document) = state.active_document() else {
                    return current_name(None);
                };
                document_title_text(&document)
            })
            .style(|s| {
                s.font_size(13.0)
                    .font_bold()
                    .color(Color::from_rgb8(44, 50, 63))
            }),
        ))
    }
    .style(|s| {
        s.width_full()
            .justify_between()
            .items_center()
            .padding_horiz(10.0)
            .padding_vert(9.0)
            .background(Color::from_rgb8(236, 232, 221))
    });

    let status_strip = {
        let state = state.clone();
        Label::derived(move || state.status_message.get().unwrap_or_default())
    }
    .style({
        let state = state.clone();
        move |s| {
            s.width_full()
                .padding_horiz(12.0)
                .padding_vert(8.0)
                .font_size(12.0)
                .color(Color::from_rgb8(82, 89, 102))
                .background(Color::from_rgb8(243, 239, 230))
                .apply_if(state.status_message.get().is_none(), |s| s.hide())
        }
    });

    let tabs_strip = tab_strip_view(state.clone());
    let tabs_content = tab_content_view(state.clone(), bootstrap.command_registry.clone());

    Stack::new((
        Stack::vertical((top_bar, tabs_strip, status_strip, tabs_content)).style(|s| {
            s.size_full()
                .padding(10.0)
                .row_gap(0.0)
                .background(Color::from_rgb8(247, 243, 233))
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
    .on_event_cont(listener::WindowCloseRequested, {
        let state = state.clone();
        move |cx, _| {
            let dirty_documents = state.dirty_document_ids_untracked();
            let Some(first_dirty_document) = dirty_documents.first().copied() else {
                return;
            };
            cx.prevent_default();
            activate_document(&state, first_dirty_document);
            state.pending_action.set(Some(PendingAction::CloseWindow {
                window_id,
                remaining_documents: dirty_documents,
            }));
            state.show_confirm.set(true);
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
