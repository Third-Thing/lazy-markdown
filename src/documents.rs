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
    views::editor::{
        Editor,
        text::{Document, SimpleStyling, Styling},
        text_document::TextDocument,
    },
    window::WindowId,
};

use crate::{
    recent_files::{record_recent_file, remove_recent_file},
    state::{AppState, DocumentId, DocumentState, PendingAction, save_target_path},
};

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
) -> DocumentState {
    let file_path = scope.create_rw_signal(file_path);
    let doc: Rc<dyn Document> = Rc::new(TextDocument::new(scope, text));
    let style: Rc<dyn Styling> = Rc::new(SimpleStyling::new());
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

fn create_document(state: &AppState, file_path: Option<PathBuf>, text: String) -> DocumentState {
    let document_id = state.allocate_document_id();
    create_document_state(state.document_scope, document_id, file_path, text)
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

pub(crate) fn create_new_tab(state: &AppState) {
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
