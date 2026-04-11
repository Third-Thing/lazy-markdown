use std::path::{Path, PathBuf};

use floem::{
    prelude::{RwSignal, SignalGet, SignalUpdate},
    reactive::Scope,
    views::editor::Editor,
};

#[derive(Clone)]
pub(crate) enum PendingAction {
    CloseDocument {
        document_id: DocumentId,
    },
    CloseWindow {
        window_id: floem::window::WindowId,
        remaining_documents: Vec<DocumentId>,
    },
}

#[derive(Clone)]
pub(crate) struct DocumentState {
    id: DocumentId,
    pub(crate) file_path: RwSignal<Option<PathBuf>>,
    pub(crate) editor: Editor,
}

impl DocumentState {
    pub(crate) fn new(
        id: DocumentId,
        file_path: RwSignal<Option<PathBuf>>,
        editor: Editor,
    ) -> Self {
        Self {
            id,
            file_path,
            editor,
        }
    }

    pub(crate) fn id(&self) -> DocumentId {
        self.id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DocumentId(u64);

impl DocumentId {
    pub(crate) fn initial() -> Self {
        Self(1)
    }

    fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Clone)]
pub(crate) struct DocumentSet {
    documents: Vec<DocumentState>,
    active_id: Option<DocumentId>,
    next_id: DocumentId,
}

impl DocumentSet {
    pub(crate) fn empty() -> Self {
        Self {
            documents: Vec::new(),
            active_id: None,
            next_id: DocumentId::initial(),
        }
    }

    pub(crate) fn new(active_document: DocumentState) -> Self {
        Self {
            active_id: Some(active_document.id()),
            next_id: active_document.id().next(),
            documents: vec![active_document],
        }
    }

    pub(crate) fn active_document(&self) -> Option<DocumentState> {
        self.active_id.and_then(|id| self.document_by_id(id))
    }

    pub(crate) fn active_document_id(&self) -> Option<DocumentId> {
        self.active_id
    }

    pub(crate) fn active_index(&self) -> Option<usize> {
        let active_id = self.active_id?;
        self.documents
            .iter()
            .position(|document| document.id() == active_id)
    }

    pub(crate) fn document_by_id(&self, id: DocumentId) -> Option<DocumentState> {
        self.documents
            .iter()
            .find(|document| document.id() == id)
            .cloned()
    }

    pub(crate) fn documents(&self) -> Vec<DocumentState> {
        self.documents.clone()
    }

    pub(crate) fn next_document_id(&mut self) -> DocumentId {
        let document_id = self.next_id;
        self.next_id = self.next_id.next();
        document_id
    }

    pub(crate) fn push_document(&mut self, document: DocumentState) {
        self.active_id = Some(document.id());
        self.documents.push(document);
    }

    pub(crate) fn set_active_document(&mut self, document_id: DocumentId) {
        if self
            .documents
            .iter()
            .any(|document| document.id() == document_id)
        {
            self.active_id = Some(document_id);
        }
    }

    pub(crate) fn remove_document(&mut self, document_id: DocumentId) -> Option<DocumentState> {
        let index = self
            .documents
            .iter()
            .position(|document| document.id() == document_id)?;
        let removed = self.documents.remove(index);

        if self.documents.is_empty() {
            self.active_id = None;
        } else if self.active_id == Some(document_id) {
            let next_index = index.min(self.documents.len() - 1);
            self.active_id = Some(self.documents[next_index].id());
        }

        Some(removed)
    }

    pub(crate) fn find_by_path(&self, path: &Path) -> Option<DocumentState> {
        self.documents
            .iter()
            .find(|document| document_path_matches(document, path))
            .cloned()
    }

    pub(crate) fn dirty_document_ids(&self) -> Vec<DocumentId> {
        self.documents
            .iter()
            .filter(|document| document.editor.doc().is_dirty())
            .map(DocumentState::id)
            .collect()
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) document_scope: Scope,
    pub(crate) documents: RwSignal<DocumentSet>,
    pub(crate) status_message: RwSignal<Option<String>>,
    pub(crate) pending_action: RwSignal<Option<PendingAction>>,
    pub(crate) show_confirm: RwSignal<bool>,
    pub(crate) save_as_dialog_open: RwSignal<bool>,
}

impl AppState {
    pub(crate) fn new(document_scope: Scope) -> Self {
        Self {
            document_scope,
            documents: RwSignal::new(DocumentSet::empty()),
            status_message: RwSignal::new(None::<String>),
            pending_action: RwSignal::new(None::<PendingAction>),
            show_confirm: RwSignal::new(false),
            save_as_dialog_open: RwSignal::new(false),
        }
    }

    pub(crate) fn active_document(&self) -> Option<DocumentState> {
        self.documents.get().active_document()
    }

    pub(crate) fn documents(&self) -> Vec<DocumentState> {
        self.documents.get().documents()
    }

    pub(crate) fn active_document_untracked(&self) -> Option<DocumentState> {
        self.documents.get_untracked().active_document()
    }

    pub(crate) fn document_by_id_untracked(&self, id: DocumentId) -> Option<DocumentState> {
        self.documents.get_untracked().document_by_id(id)
    }

    pub(crate) fn set_active_document(&self, id: DocumentId) {
        self.documents.update(|documents| {
            documents.set_active_document(id);
        });
    }

    pub(crate) fn allocate_document_id(&self) -> DocumentId {
        self.documents
            .try_update(DocumentSet::next_document_id)
            .unwrap_or_else(|| panic!("documents signal disposed while allocating a document id"))
    }

    pub(crate) fn push_document(&self, document: DocumentState) {
        self.documents.update(|documents| {
            documents.push_document(document);
        });
    }

    pub(crate) fn remove_document(&self, document_id: DocumentId) -> Option<DocumentState> {
        let mut removed = None;
        self.documents.update(|documents| {
            removed = documents.remove_document(document_id);
        });
        removed
    }

    pub(crate) fn active_index(&self) -> Option<usize> {
        self.documents.get().active_index()
    }

    pub(crate) fn find_document_by_path_untracked(&self, path: &Path) -> Option<DocumentState> {
        self.documents.get_untracked().find_by_path(path)
    }

    pub(crate) fn dirty_document_ids_untracked(&self) -> Vec<DocumentId> {
        self.documents.get_untracked().dirty_document_ids()
    }
}

pub(crate) fn save_target_path(path: &Path) -> PathBuf {
    if path.exists() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

fn document_path_matches(document: &DocumentState, path: &Path) -> bool {
    document
        .file_path
        .get_untracked()
        .is_some_and(|document_path| save_target_path(&document_path) == save_target_path(path))
}
