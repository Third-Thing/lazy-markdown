use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use gpui::{
    AppContext as _, Context, Entity, Focusable as _, PathPromptOptions, SharedString, Window,
};
use gpui_component::{
    WindowExt,
    button::ButtonVariant,
    dialog::DialogButtonProps,
    input::{InputEvent, InputState},
};

use crate::{
    persistence::{EditorMode, store_recent_files, write_file_atomic},
    window::AppWindow,
};

pub(crate) const MAX_OPEN_TABS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct DocumentId(pub(crate) u64);

impl DocumentId {
    pub(crate) fn initial() -> Self {
        Self(1)
    }

    pub(crate) fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

pub(crate) struct Document {
    pub(crate) id: DocumentId,
    pub(crate) editor: Entity<InputState>,
    pub(crate) current_path: Option<PathBuf>,
    pub(crate) pristine_text: String,
    pub(crate) dirty: bool,
}

impl Document {
    pub(crate) fn title(&self) -> SharedString {
        let marker = if self.dirty { " *" } else { "" };
        format!("{}{marker}", self.saved_title()).into()
    }

    pub(crate) fn saved_title(&self) -> String {
        current_name(self.current_path.as_deref())
    }
}

fn current_name(path: Option<&Path>) -> String {
    path.and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_string())
}

pub(crate) fn document_status(value: &str, dirty: bool) -> SharedString {
    let marker = if dirty { "modified, " } else { "" };
    format!(
        "{}{} lines, {} chars",
        marker,
        value.lines().count(),
        value.chars().count()
    )
    .into()
}

impl AppWindow {
    pub(crate) fn allocate_document_id(&mut self) -> DocumentId {
        let id = self.next_document_id;
        self.next_document_id = self.next_document_id.next();
        id
    }

    pub(crate) fn create_document(
        &mut self,
        current_path: Option<PathBuf>,
        text: String,
        status: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<DocumentId> {
        if !self.ensure_tab_capacity(window, cx) {
            return None;
        }

        let id = self.allocate_document_id();
        let editor_mode = self.app_config.editor_mode;
        let editor = cx.new(|cx| create_editor_state(text.clone(), editor_mode, window, cx));
        let subscription = cx.subscribe_in(&editor, window, {
            let editor = editor.clone();
            move |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.refresh_document_status(id, &editor, cx);
                }
            }
        });

        self._subscriptions.push(subscription);
        self.documents.push(Document {
            id,
            editor,
            current_path,
            pristine_text: text,
            dirty: false,
        });
        self.activate_document(id, window, cx);
        self.status = status;
        cx.notify();

        Some(id)
    }

    pub(crate) fn create_new_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.create_document(
            None,
            String::new(),
            "Started a new document".into(),
            window,
            cx,
        );
    }

    pub(crate) fn prompt_open_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open Markdown file".into()),
        });
        let view = cx.entity();

        cx.spawn_in(window, async move |_, window| {
            let Some(path) = receiver.await.ok().and_then(Result::ok).flatten() else {
                return;
            };
            let Some(path) = path.into_iter().next() else {
                return;
            };
            let contents = std::fs::read_to_string(&path);

            _ = window.update(|window, cx| {
                _ = view.update(cx, |this, cx| match contents {
                    Ok(contents) => {
                        this.open_document_path(path.clone(), contents, window, cx);
                    }
                    Err(err) => {
                        this.status = format!("Failed to open {}: {err}", path.display()).into();
                        cx.notify();
                    }
                });
            });
        })
        .detach();
    }

    pub(crate) fn open_document_path(
        &mut self,
        path: PathBuf,
        contents: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(document_id) = self.document_id_for_path(&path) {
            self.activate_document(document_id, window, cx);
            if self.record_recent_file(&path, cx) {
                self.status = format!("Switched to {}", path.display()).into();
            }
            cx.notify();
            return;
        }

        if self
            .create_document(
                Some(path.clone()),
                contents,
                format!("Opened {}", path.display()).into(),
                window,
                cx,
            )
            .is_some()
        {
            self.record_recent_file(&path, cx);
        }
    }

    pub(crate) fn open_document_path_from_disk(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(document_id) = self.document_id_for_path(&path) {
            self.activate_document(document_id, window, cx);
            if self.record_recent_file(&path, cx) {
                self.status = format!("Switched to {}", path.display()).into();
            }
            cx.notify();
            return;
        }

        if !self.ensure_tab_capacity(window, cx) {
            return;
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => {
                self.create_document(
                    Some(path.clone()),
                    contents,
                    format!("Opened {}", path.display()).into(),
                    window,
                    cx,
                );
                self.record_recent_file(&path, cx);
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    self.remove_recent_file(&path, cx);
                }
                self.status = format!("Failed to open {}: {err}", path.display()).into();
                cx.notify();
            }
        }
    }

    pub(crate) fn save_to_path(
        &mut self,
        document_id: DocumentId,
        path: &Path,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.document_index(document_id) else {
            return;
        };
        let contents = self.documents[index].editor.read(cx).value().to_string();
        match write_file_atomic(path, contents.as_bytes()) {
            Ok(()) => {
                let document = &mut self.documents[index];
                document.current_path = Some(path.to_path_buf());
                document.pristine_text = contents;
                document.dirty = false;
                if self.record_recent_file(path, cx) {
                    self.status = format!("Saved {}", path.display()).into();
                }
            }
            Err(err) => {
                self.status = format!("Failed to save {}: {err}", path.display()).into();
            }
        }
        cx.notify();
    }

    pub(crate) fn prompt_save_as(
        &mut self,
        document_id: DocumentId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(document) = self.document_by_id(document_id) else {
            return;
        };
        let directory = document
            .current_path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let suggested_name = document
            .current_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("untitled.md")
            .to_string();
        let contents = document.editor.read(cx).value().to_string();
        let receiver = cx.prompt_for_new_path(&directory, Some(&suggested_name));
        let view = cx.entity();

        cx.spawn_in(window, async move |_, window| {
            let Some(path) = receiver.await.ok().and_then(Result::ok).flatten() else {
                return;
            };
            let result = write_file_atomic(&path, contents.as_bytes());

            _ = window.update(|_, cx| {
                _ = view.update(cx, |this, cx| {
                    match result {
                        Ok(()) => {
                            if let Some(index) = this.document_index(document_id) {
                                let document = &mut this.documents[index];
                                document.current_path = Some(path.clone());
                                document.pristine_text = contents.clone();
                                document.dirty = false;
                                if this.record_recent_file(&path, cx) {
                                    this.status = format!("Saved {}", path.display()).into();
                                }
                            }
                        }
                        Err(err) => {
                            this.status =
                                format!("Failed to save {}: {err}", path.display()).into();
                        }
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }

    pub(crate) fn record_recent_file(&mut self, path: &Path, cx: &mut Context<Self>) -> bool {
        if !self.recent_files.add_path(path) {
            return true;
        }

        if let Err(err) = store_recent_files(&self.recent_files) {
            self.status = err.into();
            cx.notify();
            return false;
        }

        self.reload_app_menus(cx);
        true
    }

    fn remove_recent_file(&mut self, path: &Path, cx: &mut Context<Self>) {
        if !self.recent_files.remove_path(path) {
            return;
        }

        if let Err(err) = store_recent_files(&self.recent_files) {
            self.status = err.into();
            cx.notify();
            return;
        }

        self.reload_app_menus(cx);
    }

    pub(crate) fn clear_recent_files(&mut self, cx: &mut Context<Self>) {
        if !self.recent_files.clear() {
            return;
        }

        if let Err(err) = store_recent_files(&self.recent_files) {
            self.status = err.into();
            cx.notify();
            return;
        }

        self.reload_app_menus(cx);
        self.status = "Cleared recent files".into();
        cx.notify();
    }

    pub(crate) fn request_close_document(
        &mut self,
        document_id: DocumentId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_document(document_id, window, cx);
        let Some(document) = self.document_by_id(document_id) else {
            return;
        };

        if document.dirty {
            self.open_close_document_dialog(document_id, window, cx);
        } else {
            self.close_document_now(document_id, window, cx);
        }
    }

    fn open_close_document_dialog(
        &mut self,
        document_id: DocumentId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let title = self
            .document_by_id(document_id)
            .map(Document::saved_title)
            .unwrap_or_else(|| "Untitled".to_string());
        let description = format!("{title} has unsaved changes. Close without saving?");
        let view = cx.entity();

        window.open_alert_dialog(cx, move |dialog, _, _| {
            let view = view.clone();

            dialog
                .title("Discard changes?")
                .description(description.clone())
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Discard")
                        .ok_variant(ButtonVariant::Danger)
                        .cancel_text("Keep Editing")
                        .show_cancel(true),
                )
                .on_ok(move |_, window, cx| {
                    let view = view.clone();
                    window.defer(cx, move |window, cx| {
                        _ = view.update(cx, |this, cx| {
                            this.close_document_now(document_id, window, cx);
                        });
                    });

                    true
                })
        });
    }

    fn close_document_now(
        &mut self,
        document_id: DocumentId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.document_index(document_id) else {
            return;
        };

        if self.documents.len() == 1 {
            let title = self.documents[index].saved_title();
            self.documents[index].current_path = None;
            self.documents[index].pristine_text.clear();
            self.documents[index].dirty = false;
            self.documents[index].editor.update(cx, |editor, cx| {
                editor.set_value("", window, cx);
            });
            self.status = format!("Closed {title}").into();
            self.focus_active_document(window, cx);
            cx.notify();
            return;
        }

        let title = self.documents[index].saved_title();
        self.documents.remove(index);
        if self.active_document_id == Some(document_id) {
            let next_index = index.min(self.documents.len() - 1);
            self.active_document_id = Some(self.documents[next_index].id);
        }
        self.status = format!("Closed {title}").into();
        self.focus_active_document(window, cx);
        cx.notify();
    }

    fn ensure_tab_capacity(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.documents.len() < MAX_OPEN_TABS {
            return true;
        }

        let description = format!(
            "You can only keep {MAX_OPEN_TABS} tabs open at once. Close a tab before opening another file or creating a new document."
        );
        self.status = "Tab limit reached".into();
        cx.notify();

        if !window.has_active_dialog(cx) {
            window.open_alert_dialog(cx, move |dialog, _, _| {
                dialog
                    .title("Tab limit reached")
                    .description(description.clone())
                    .button_props(DialogButtonProps::default().ok_text("OK"))
            });
        }

        false
    }

    pub(crate) fn activate_document(
        &mut self,
        document_id: DocumentId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.document_by_id(document_id).is_none() {
            return;
        }

        self.active_document_id = Some(document_id);
        self.focus_active_document(window, cx);
        cx.notify();
    }

    pub(crate) fn activate_index(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(document) = self.documents.get(index) {
            self.activate_document(document.id, window, cx);
        }
    }

    fn focus_active_document(&self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(document) = self.active_document() else {
            return;
        };
        let focus_handle = document.editor.focus_handle(cx);
        window.defer(cx, move |window, cx| {
            focus_handle.focus(window, cx);
        });
    }

    fn refresh_document_status(
        &mut self,
        document_id: DocumentId,
        editor: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self.document_index(document_id) else {
            return;
        };
        let value = editor.read(cx).value();
        let document = &mut self.documents[index];
        document.dirty = value.as_ref() != document.pristine_text;

        if self.active_document_id == Some(document_id) {
            self.status = document_status(value.as_ref(), document.dirty);
        }

        cx.notify();
    }

    pub(crate) fn dirty_document_ids(&self) -> Vec<DocumentId> {
        self.documents
            .iter()
            .filter(|document| document.dirty)
            .map(|document| document.id)
            .collect()
    }

    pub(crate) fn active_index(&self) -> Option<usize> {
        let active_document_id = self.active_document_id?;
        self.document_index(active_document_id)
    }

    pub(crate) fn active_document(&self) -> Option<&Document> {
        self.active_document_id
            .and_then(|document_id| self.document_by_id(document_id))
    }

    pub(crate) fn active_editor(&self) -> Option<Entity<InputState>> {
        self.active_document()
            .map(|document| document.editor.clone())
    }

    pub(crate) fn document_by_id(&self, document_id: DocumentId) -> Option<&Document> {
        self.documents
            .iter()
            .find(|document| document.id == document_id)
    }

    fn document_index(&self, document_id: DocumentId) -> Option<usize> {
        self.documents
            .iter()
            .position(|document| document.id == document_id)
    }

    fn document_id_for_path(&self, path: &Path) -> Option<DocumentId> {
        self.documents
            .iter()
            .find(|document| document.current_path.as_deref() == Some(path))
            .map(|document| document.id)
    }
}

fn create_editor_state(
    text: String,
    editor_mode: EditorMode,
    window: &mut Window,
    cx: &mut Context<InputState>,
) -> InputState {
    let input = InputState::new(window, cx);
    let input = match editor_mode {
        EditorMode::Basic => input.multi_line(true),
        EditorMode::CodeEditor => input.code_editor("markdown"),
    };

    input.searchable(true).default_value(text)
}
