use floem::{
    prelude::*,
    reactive::Effect,
    views::{
        editor::{keypress::default_key_handler, view::editor_container_view},
        tab,
    },
};

use crate::preferences::theme::editor_theme_style;

use super::state::{AppState, DocumentState};

fn document_editor_view(document: DocumentState, state: AppState) -> impl IntoView {
    let editor_sig = RwSignal::new(document.editor.clone());
    let document_id = document.id();
    let focus_document = document.clone();
    let focus_state = state.clone();

    Effect::new(move |_| {
        let is_active = focus_state.documents.get().active_document_id() == Some(document_id);
        let view_id = focus_document.editor.editor_view_id.get();

        if is_active && let Some(view_id) = view_id {
            view_id.request_focus();
        }
    });

    editor_container_view(editor_sig, |_| true, default_key_handler(editor_sig)).style({
        let state = state.clone();
        move |s| {
            let theme = state.app_theme();
            s.apply(editor_theme_style(theme))
                .width_full()
                .min_size(0, 0)
                .flex_grow(1.0)
                .border(1.0)
                .border_color(theme.border)
        }
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

#[cfg(test)]
mod tests {
    use floem::{
        headless::{HeadlessHarness, TestRoot},
        prelude::{SignalGet, SignalUpdate},
        reactive::Scope,
    };

    use crate::{
        persistence::{config::AppConfig, recent_files::RecentFiles},
        workspace::{AppState, DocumentId, DocumentSet, activate_document, create_document_state},
    };

    use super::tab_content_view;

    #[test]
    fn tab_content_view_builds_editor_views_and_tracks_active_document() {
        let root = TestRoot::new();
        let state = test_state_with_two_documents();
        let document_a = state
            .document_by_id_untracked(DocumentId::initial())
            .expect("document a");
        let document_b = state
            .documents()
            .into_iter()
            .find(|document| document.id() != DocumentId::initial())
            .expect("document b");
        let mut harness =
            HeadlessHarness::new_with_size(root, tab_content_view(state.clone()), 920.0, 680.0);

        harness.rebuild();

        let editor_a_view_id = document_a
            .editor
            .editor_view_id
            .get_untracked()
            .expect("editor a view id");
        let editor_b_view_id = document_b
            .editor
            .editor_view_id
            .get_untracked()
            .expect("editor b view id");
        assert_ne!(editor_a_view_id, editor_b_view_id);
        assert_eq!(state.active_index(), Some(0));

        state.set_active_document(document_b.id());
        harness.process_update_no_paint();

        assert_eq!(state.active_index(), Some(1));
    }

    fn test_state_with_two_documents() -> AppState {
        let scope = Scope::new();
        let state = AppState::new(scope, RecentFiles::default(), AppConfig::default());
        let initial_document = create_document_state(
            scope,
            DocumentId::initial(),
            None,
            String::from("first"),
            state.editor_font_untracked(),
            state.editor_font_size_untracked(),
        );
        state.documents.set(DocumentSet::new(initial_document));

        let second_document = create_document_state(
            state.document_scope,
            state.allocate_document_id(),
            None,
            String::from("second"),
            state.editor_font_untracked(),
            state.editor_font_size_untracked(),
        );
        state.push_document(second_document);
        activate_document(&state, DocumentId::initial());
        state
    }
}
