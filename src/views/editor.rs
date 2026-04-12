use floem::{
    peniko::Color,
    prelude::*,
    reactive::Effect,
    views::{
        editor::{keypress::default_key_handler, view::editor_container_view},
        tab,
    },
};

use crate::{
    state::{AppState, DocumentState},
    theme::editor_theme_style,
};

fn document_editor_view(document: DocumentState, state: AppState) -> impl IntoView {
    let editor_sig = RwSignal::new(document.editor.clone());
    let document_id = document.id();
    let focus_document = document.clone();
    let focus_state = state;

    Effect::new(move |_| {
        let is_active = focus_state.documents.get().active_document_id() == Some(document_id);
        let view_id = focus_document.editor.editor_view_id.get();

        if is_active && let Some(view_id) = view_id {
            view_id.request_focus();
        }
    });

    editor_container_view(editor_sig, |_| true, default_key_handler(editor_sig)).style(|s| {
        s.apply(editor_theme_style())
            .width_full()
            .min_size(0, 0)
            .flex_grow(1.0)
            .border(1.0)
            .border_color(Color::from_rgb8(220, 223, 227))
    })
}

pub(crate) fn tab_content_view(state: AppState) -> impl IntoView {
    let active_state = state.clone();
    let documents_state = state.clone();

    tab(
        move || active_state.active_index(),
        move || documents_state.documents(),
        DocumentState::id,
        move |document| document_editor_view(document, state.clone()),
    )
    .style(|s| s.width_full().min_size(0, 0).flex_grow(1.0))
}
