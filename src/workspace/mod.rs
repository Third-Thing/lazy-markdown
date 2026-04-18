pub(crate) mod documents;
pub(crate) mod editor_area;
pub(crate) mod state;
pub(crate) mod tabs;

pub(crate) use documents::{
    activate_document, create_document_state, create_new_tab, current_name, document_title_text,
    finish_pending_action, focus_active_document, open_document_path, request_open, request_save,
    request_save_as,
};
pub(crate) use editor_area::tab_content_view;
pub(crate) use state::{
    AppState, DocumentId, DocumentSet, MenuUiState, PendingAction, TopLevelMenuId,
    save_target_path,
};
pub(crate) use tabs::tab_strip_view;
