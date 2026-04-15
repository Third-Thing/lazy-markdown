use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

use atomic_write_file::AtomicWriteFile;
use floem::{
    FileDialogOptions, close_window, open_file,
    prelude::{SignalGet, SignalUpdate},
    reactive::Scope,
    save_as,
    views::editor::{Editor, text::Document, text_document::TextDocument},
    window::WindowId,
};

use crate::{
    editor_font::editor_styling,
    recent_files::{record_recent_file, remove_recent_file},
    state::{AppState, DocumentId, DocumentState, PendingAction, save_target_path},
};

const MAX_OPEN_TABS: usize = 5;

pub(crate) fn current_name(path: Option<&Path>) -> String {
    path.and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_string())
}

pub(crate) fn create_document_state(
    scope: Scope,
    document_id: DocumentId,
    file_path: Option<PathBuf>,
    text: String,
    editor_font: String,
    editor_font_size: usize,
) -> DocumentState {
    let file_path = scope.create_rw_signal(file_path);
    let doc: Rc<dyn Document> = Rc::new(TextDocument::new(scope, text));
    let style = editor_styling(&editor_font, editor_font_size);
    let editor = Editor::new(scope, doc, style, false);
    DocumentState::new(document_id, file_path, editor)
}

pub(crate) fn save_document_title(document: &DocumentState) -> String {
    let path = document.file_path.get_untracked();
    current_name(path.as_deref())
}

pub(crate) fn document_title_text(document: &DocumentState) -> String {
    let path = document.file_path.get();
    let modified = document
        .editor
        .try_doc()
        .map(|doc| if doc.dirty().get() { " *" } else { "" })
        .unwrap_or("");
    format!("{}{}", current_name(path.as_deref()), modified)
}

fn write_editor_text(editor: &Editor, path: &Path) -> Result<(), String> {
    let save_path = save_target_path(path);
    let file = AtomicWriteFile::open(&save_path)
        .map_err(|err| format!("Failed to save {}: {err}", path.display()))?;
    let mut writer = BufWriter::new(file);
    let text = editor.doc().text();

    for chunk in text.iter_chunks(..) {
        writer
            .write_all(chunk.as_bytes())
            .map_err(|err| format!("Failed to save {}: {err}", path.display()))?;
    }

    writer
        .flush()
        .map_err(|err| format!("Failed to save {}: {err}", path.display()))?;
    let file = writer
        .into_inner()
        .map_err(|err| format!("Failed to save {}: {}", path.display(), err.into_error()))?;
    file.commit()
        .map_err(|err| format!("Failed to save {}: {err}", path.display()))
}

fn save_document_to_path(state: &AppState, document: &DocumentState, path: &Path) {
    match write_editor_text(&document.editor, path) {
        Ok(()) => {
            document.editor.doc().mark_pristine();
            document.file_path.set(Some(path.to_path_buf()));
            record_recent_file(state, path);
            state
                .status_message
                .set(Some(format!("Saved {}", path.display())));
            if let Some(action) = state.pending_action.get_untracked() {
                finish_pending_action(action, state);
            }
        }
        Err(err) => state.status_message.set(Some(err)),
    }
}

fn replace_with_new_document(state: &AppState, document: &DocumentState, message: &str) {
    let doc: Rc<dyn Document> = Rc::new(TextDocument::new(document.editor.cx.get(), String::new()));
    document.editor.update_doc(doc, None);
    document.file_path.set(None);
    state.status_message.set(Some(message.to_string()));
}

pub(crate) fn activate_document(state: &AppState, document_id: DocumentId) {
    state.set_active_document(document_id);

    if let Some(document) = state.document_by_id_untracked(document_id)
        && let Some(view_id) = document.editor.editor_view_id.get_untracked()
    {
        view_id.request_focus();
    }
}

pub(crate) fn focus_active_document(state: &AppState) {
    let Some(document_id) = state
        .active_document_untracked()
        .map(|document| document.id())
    else {
        return;
    };

    activate_document(state, document_id);
}

fn create_document(state: &AppState, file_path: Option<PathBuf>, text: String) -> DocumentState {
    let document_id = state.allocate_document_id();
    create_document_state(
        state.document_scope,
        document_id,
        file_path,
        text,
        state.editor_font_untracked(),
        state.editor_font_size_untracked(),
    )
}

fn create_and_activate_document(
    state: &AppState,
    file_path: Option<PathBuf>,
    text: String,
) -> DocumentState {
    let document = create_document(state, file_path, text);
    state.push_document(document.clone());
    document
}

fn show_tab_limit_dialog(state: &AppState) {
    state.pending_action.set(Some(PendingAction::ShowMessage {
        title: "Tab limit reached".to_string(),
        message: format!(
            "You can only keep {MAX_OPEN_TABS} tabs open at once. Close a tab before opening another file or creating a new document."
        ),
    }));
    state.show_confirm.set(true);
}

fn ensure_tab_capacity(state: &AppState) -> bool {
    if state.document_count_untracked() < MAX_OPEN_TABS {
        return true;
    }

    show_tab_limit_dialog(state);
    false
}

pub(crate) fn create_new_tab(state: &AppState) {
    if !ensure_tab_capacity(state) {
        return;
    }

    let document = create_and_activate_document(state, None, String::new());
    activate_document(state, document.id());
    state
        .status_message
        .set(Some("Started a new document".to_string()));
}

pub(crate) fn open_document_path(state: &AppState, path: PathBuf) {
    if let Some(document) = state.find_document_by_path_untracked(&path) {
        activate_document(state, document.id());
        record_recent_file(state, &path);
        state
            .status_message
            .set(Some(format!("Switched to {}", path.display())));
        return;
    }

    if !ensure_tab_capacity(state) {
        return;
    }

    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let document = create_and_activate_document(state, Some(path.clone()), text);
            activate_document(state, document.id());
            record_recent_file(state, &path);
            state
                .status_message
                .set(Some(format!("Opened {}", path.display())));
        }
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                remove_recent_file(state, &path);
            }
            state
                .status_message
                .set(Some(format!("Failed to open {}: {err}", path.display())));
        }
    }
}

fn close_document_now(state: &AppState, document_id: DocumentId) {
    let Some(document) = state.document_by_id_untracked(document_id) else {
        return;
    };

    if state.documents().len() == 1 {
        replace_with_new_document(state, &document, "Started a new document");
        return;
    }

    let title = save_document_title(&document);
    state.remove_document(document_id);
    state.status_message.set(Some(format!("Closed {title}")));
}

pub(crate) fn request_close_document(state: &AppState, document_id: DocumentId) {
    let Some(document) = state.document_by_id_untracked(document_id) else {
        return;
    };

    activate_document(state, document_id);

    if document.editor.doc().is_dirty() {
        state
            .pending_action
            .set(Some(PendingAction::CloseDocument { document_id }));
        state.show_confirm.set(true);
    } else {
        close_document_now(state, document_id);
    }
}

fn advance_window_close(
    window_id: WindowId,
    remaining_documents: Vec<DocumentId>,
    state: &AppState,
) {
    let remaining_documents: Vec<DocumentId> = remaining_documents
        .into_iter()
        .skip(1)
        .filter(|document_id| state.document_by_id_untracked(*document_id).is_some())
        .collect();

    if let Some(next_document_id) = remaining_documents.first().copied() {
        activate_document(state, next_document_id);
        state.pending_action.set(Some(PendingAction::CloseWindow {
            window_id,
            remaining_documents,
        }));
        state.show_confirm.set(true);
    } else {
        if let Err(err) = state.store_app_config() {
            eprintln!("Failed to save settings on exit: {err}");
        }
        state.pending_action.set(None);
        state.show_confirm.set(false);
        close_window(window_id);
    }
}

pub(crate) fn finish_pending_action(action: PendingAction, state: &AppState) {
    state.pending_action.set(None);
    state.show_confirm.set(false);

    match action {
        PendingAction::CloseDocument { document_id } => {
            close_document_now(state, document_id);
        }
        PendingAction::CloseWindow {
            window_id,
            remaining_documents,
        } => {
            advance_window_close(window_id, remaining_documents, state);
        }
        PendingAction::ShowMessage { .. } => {}
    }
}

pub(crate) fn request_save_as(state: AppState, document: DocumentState) {
    if state.save_as_dialog_open.get_untracked() {
        return;
    }

    state.save_as_dialog_open.set(true);

    let mut options = FileDialogOptions::new()
        .title("Save file")
        .default_name(current_name(document.file_path.get_untracked().as_deref()));

    if let Some(path) = document
        .file_path
        .get_untracked()
        .as_ref()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        options = options.force_starting_directory(path);
    }

    save_as(options, move |file_info| {
        let Some(path) = file_info.and_then(|info| info.path.into_iter().next()) else {
            state.save_as_dialog_open.set(false);
            return;
        };

        save_document_to_path(&state, &document, &path);
        state.save_as_dialog_open.set(false);
    });
}

pub(crate) fn request_save(state: &AppState, document: &DocumentState) {
    if let Some(path) = document.file_path.get_untracked() {
        save_document_to_path(state, document, &path);
    } else {
        request_save_as(state.clone(), document.clone());
    }
}

pub(crate) fn request_open(state: AppState) {
    if !ensure_tab_capacity(&state) {
        return;
    }

    let mut options = FileDialogOptions::new().title("Open file");
    if let Some(path) = state
        .active_document_untracked()
        .and_then(|document| document.file_path.get_untracked())
        .as_ref()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        options = options.force_starting_directory(path);
    }

    open_file(options, move |file_info| {
        let Some(path) = file_info.and_then(|info| info.path.into_iter().next()) else {
            return;
        };

        open_document_path(&state, path);
    });
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use floem::{
        headless::TestRoot,
        prelude::{SignalGet, SignalUpdate},
        reactive::Scope,
    };

    use crate::{
        config::AppConfig,
        recent_files::RecentFiles,
        state::{AppState, DocumentId, DocumentSet, PendingAction},
    };

    use super::{MAX_OPEN_TABS, create_document_state, create_new_tab, open_document_path};

    #[test]
    fn create_new_tab_shows_popup_at_limit() {
        let _root = TestRoot::new();
        let state = test_state_with_document_count(MAX_OPEN_TABS);

        create_new_tab(&state);

        assert_eq!(state.document_count_untracked(), MAX_OPEN_TABS);
        assert!(state.show_confirm.get_untracked());
        assert!(matches!(
            state.pending_action.get_untracked(),
            Some(PendingAction::ShowMessage { .. })
        ));
    }

    #[test]
    fn opening_new_file_shows_popup_at_limit() {
        let _root = TestRoot::new();
        let state = test_state_with_document_count(MAX_OPEN_TABS);
        let path = temp_markdown_file("open-limit", "opened from disk");

        open_document_path(&state, path.clone());

        assert_eq!(state.document_count_untracked(), MAX_OPEN_TABS);
        assert!(state.show_confirm.get_untracked());
        assert!(matches!(
            state.pending_action.get_untracked(),
            Some(PendingAction::ShowMessage { .. })
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn opening_existing_file_switches_tabs_even_at_limit() {
        let _root = TestRoot::new();
        let state = test_state_with_document_count(MAX_OPEN_TABS);
        let target_path = temp_markdown_file("already-open", "opened from disk");
        let target_document = create_document_state(
            state.document_scope,
            state.allocate_document_id(),
            Some(target_path.clone()),
            String::from("already open"),
            state.editor_font_untracked(),
            state.editor_font_size_untracked(),
        );
        let target_document_id = target_document.id();

        let removed = state.remove_document(DocumentId::initial());
        assert!(removed.is_some());
        state.push_document(target_document);
        assert_eq!(state.document_count_untracked(), MAX_OPEN_TABS);

        open_document_path(&state, target_path.clone());

        assert_eq!(
            state.documents.get_untracked().active_document_id(),
            Some(target_document_id)
        );
        assert!(!state.show_confirm.get_untracked());
        assert!(state.pending_action.get_untracked().is_none());

        let _ = fs::remove_file(target_path);
    }

    fn test_state_with_document_count(count: usize) -> AppState {
        let scope = Scope::new();
        let state = AppState::new(scope, RecentFiles::default(), AppConfig::default());
        let initial_document = create_document_state(
            scope,
            DocumentId::initial(),
            None,
            String::from("initial"),
            state.editor_font_untracked(),
            state.editor_font_size_untracked(),
        );
        state.documents.set(DocumentSet::new(initial_document));

        for index in 1..count {
            let document = create_document_state(
                state.document_scope,
                state.allocate_document_id(),
                None,
                format!("document {index}"),
                state.editor_font_untracked(),
                state.editor_font_size_untracked(),
            );
            state.push_document(document);
        }

        state
    }

    fn temp_markdown_file(prefix: &str, contents: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{unique}.md"));
        fs::write(&path, contents).expect("write temp file");
        path
    }
}
