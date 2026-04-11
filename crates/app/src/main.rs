use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

use atomic_write_file::AtomicWriteFile;
use floem::{
    Application, FileDialogOptions, close_window,
    menu::Menu,
    open_file,
    peniko::{Color, color::palette},
    prelude::*,
    reactive::Scope,
    save_as,
    style::{CursorColor, Style},
    views::{
        Button, Empty, Label, Overlay, Stack, dyn_stack,
        editor::{
            CurrentLineColor, Editor, IndentGuideColor, PreeditUnderlineColor, SelectionColor,
            VisibleWhitespaceColor,
            command::CommandExecuted,
            gutter::{DimColor, GutterClass},
            keypress::{KeypressKey, KeypressMap},
            text::{Document, SimpleStyling, Styling},
            text_document::TextDocument,
            view::{EditorViewClass, editor_container_view},
        },
        tab,
    },
    window::{WindowConfig, WindowId},
};
use lazy_markdown_core::{
    CommandRegistry, ModuleRegistry, Shortcut, ShortcutKey, ShortcutModifier, command_ids,
    register_builtin_commands,
};

#[derive(Clone)]
enum PendingAction {
    CloseDocument {
        document_id: DocumentId,
    },
    CloseWindow {
        window_id: WindowId,
        remaining_documents: Vec<DocumentId>,
    },
}

#[derive(Clone)]
struct AppBootstrap {
    command_registry: CommandRegistry,
}

impl AppBootstrap {
    fn load() -> Result<Self, String> {
        let mut registry = ModuleRegistry::new();
        register_builtin_commands(registry.commands_mut())?;

        Ok(Self {
            command_registry: registry.commands().clone(),
        })
    }
}

#[derive(Clone)]
struct DocumentState {
    id: DocumentId,
    file_path: RwSignal<Option<PathBuf>>,
    editor: Editor,
}

impl DocumentState {
    fn new(id: DocumentId, file_path: RwSignal<Option<PathBuf>>, editor: Editor) -> Self {
        Self {
            id,
            file_path,
            editor,
        }
    }

    fn id(&self) -> DocumentId {
        self.id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DocumentId(u64);

impl DocumentId {
    fn initial() -> Self {
        Self(1)
    }

    fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Clone)]
struct DocumentSet {
    documents: Vec<DocumentState>,
    active_id: Option<DocumentId>,
    next_id: DocumentId,
}

impl DocumentSet {
    fn empty() -> Self {
        Self {
            documents: Vec::new(),
            active_id: None,
            next_id: DocumentId::initial(),
        }
    }

    fn new(active_document: DocumentState) -> Self {
        Self {
            active_id: Some(active_document.id()),
            next_id: active_document.id().next(),
            documents: vec![active_document],
        }
    }

    fn active_document(&self) -> Option<DocumentState> {
        self.active_id.and_then(|id| self.document_by_id(id))
    }

    fn active_document_id(&self) -> Option<DocumentId> {
        self.active_id
    }

    fn active_index(&self) -> Option<usize> {
        let active_id = self.active_id?;
        self.documents
            .iter()
            .position(|document| document.id() == active_id)
    }

    fn document_by_id(&self, id: DocumentId) -> Option<DocumentState> {
        self.documents
            .iter()
            .find(|document| document.id() == id)
            .cloned()
    }

    fn documents(&self) -> Vec<DocumentState> {
        self.documents.clone()
    }

    fn next_document_id(&mut self) -> DocumentId {
        let document_id = self.next_id;
        self.next_id = self.next_id.next();
        document_id
    }

    fn push_document(&mut self, document: DocumentState) {
        self.active_id = Some(document.id());
        self.documents.push(document);
    }

    fn set_active_document(&mut self, document_id: DocumentId) {
        if self
            .documents
            .iter()
            .any(|document| document.id() == document_id)
        {
            self.active_id = Some(document_id);
        }
    }

    fn remove_document(&mut self, document_id: DocumentId) -> Option<DocumentState> {
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

    fn find_by_path(&self, path: &Path) -> Option<DocumentState> {
        self.documents
            .iter()
            .find(|document| document_path_matches(document, path))
            .cloned()
    }

    fn dirty_document_ids(&self) -> Vec<DocumentId> {
        self.documents
            .iter()
            .filter(|document| document.editor.doc().is_dirty())
            .map(DocumentState::id)
            .collect()
    }
}

#[derive(Clone)]
struct AppState {
    document_scope: Scope,
    documents: RwSignal<DocumentSet>,
    status_message: RwSignal<Option<String>>,
    pending_action: RwSignal<Option<PendingAction>>,
    show_confirm: RwSignal<bool>,
    save_as_dialog_open: RwSignal<bool>,
}

impl AppState {
    fn active_document(&self) -> Option<DocumentState> {
        self.documents.get().active_document()
    }

    fn documents(&self) -> Vec<DocumentState> {
        self.documents.get().documents()
    }

    fn active_document_untracked(&self) -> Option<DocumentState> {
        self.documents.get_untracked().active_document()
    }

    fn document_by_id_untracked(&self, id: DocumentId) -> Option<DocumentState> {
        self.documents.get_untracked().document_by_id(id)
    }

    fn set_active_document(&self, id: DocumentId) {
        self.documents.update(|documents| {
            documents.set_active_document(id);
        });
    }

    fn allocate_document_id(&self) -> DocumentId {
        self.documents
            .try_update(DocumentSet::next_document_id)
            .unwrap_or_else(|| panic!("documents signal disposed while allocating a document id"))
    }

    fn push_document(&self, document: DocumentState) {
        self.documents.update(|documents| {
            documents.push_document(document);
        });
    }

    fn remove_document(&self, document_id: DocumentId) -> Option<DocumentState> {
        let mut removed = None;
        self.documents.update(|documents| {
            removed = documents.remove_document(document_id);
        });
        removed
    }

    fn active_index(&self) -> Option<usize> {
        self.documents.get().active_index()
    }

    fn find_document_by_path_untracked(&self, path: &Path) -> Option<DocumentState> {
        self.documents.get_untracked().find_by_path(path)
    }

    fn dirty_document_ids_untracked(&self) -> Vec<DocumentId> {
        self.documents.get_untracked().dirty_document_ids()
    }
}

#[derive(Clone)]
struct AppMenuModel {
    id: &'static str,
    title: String,
    entries: Vec<AppMenuEntry>,
}

impl AppMenuModel {
    fn new(id: &'static str, title: impl Into<String>, entries: Vec<AppMenuEntry>) -> Self {
        Self {
            id,
            title: title.into(),
            entries,
        }
    }
}

#[derive(Clone)]
enum AppMenuEntry {
    Separator,
    Item(AppMenuItem),
}

impl AppMenuEntry {
    fn item(title: impl Into<String>, action: impl Fn(&AppState) + 'static) -> Self {
        Self::Item(AppMenuItem::new(title, action))
    }

    fn disabled(title: impl Into<String>) -> Self {
        Self::Item(AppMenuItem::disabled(title))
    }
}

#[derive(Clone)]
struct AppMenuItem {
    title: String,
    enabled: bool,
    action: Option<Rc<dyn Fn(&AppState)>>,
}

impl AppMenuItem {
    fn new(title: impl Into<String>, action: impl Fn(&AppState) + 'static) -> Self {
        Self {
            title: title.into(),
            enabled: true,
            action: Some(Rc::new(action)),
        }
    }

    fn disabled(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            enabled: false,
            action: None,
        }
    }
}

fn current_name(path: Option<&Path>) -> String {
    path.and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn save_target_path(path: &Path) -> PathBuf {
    if path.exists() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

fn document_path_matches(document: &DocumentState, path: &Path) -> bool {
    document
        .file_path
        .try_get_untracked()
        .flatten()
        .is_some_and(|document_path| save_target_path(&document_path) == save_target_path(path))
}

fn create_document_state(
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

fn editor_theme_style() -> Style {
    let fg = Color::from_rgb8(0x38, 0x3A, 0x42);
    let bg = Color::from_rgb8(0xFA, 0xFA, 0xFA);
    let grey = Color::from_rgb8(0xE5, 0xE5, 0xE6);
    let dim = Color::from_rgb8(0xA0, 0xA1, 0xA7);
    let cursor = Color::from_rgb8(0x52, 0x6F, 0xFF);
    let current_line = Color::from_rgb8(0xF2, 0xF2, 0xF2);

    Style::new()
        .color(fg)
        .background(bg)
        .class(GutterClass, move |s| {
            s.background(bg)
                .set(DimColor, Some(dim))
                .set(CurrentLineColor, current_line)
        })
        .class(EditorViewClass, move |s| {
            s.set(CursorColor, cursor)
                .set(SelectionColor, grey)
                .set(CurrentLineColor, current_line)
                .set(VisibleWhitespaceColor, grey)
                .set(PreeditUnderlineColor, fg)
                .set(IndentGuideColor, grey)
        })
}

fn save_document_title(document: &DocumentState) -> String {
    let path = document.file_path.try_get_untracked().flatten();
    current_name(path.as_deref())
}

fn document_title_text(document: &DocumentState) -> String {
    let path = document.file_path.try_get().flatten();
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

fn activate_document(state: &AppState, document_id: DocumentId) {
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

fn create_new_tab(state: &AppState) {
    let document = create_and_activate_document(state, None, String::new());
    activate_document(state, document.id());
    state
        .status_message
        .set(Some("Started a new document".to_string()));
}

fn open_document_in_tab(state: &AppState, path: PathBuf) {
    if let Some(document) = state.find_document_by_path_untracked(&path) {
        activate_document(state, document.id());
        state
            .status_message
            .set(Some(format!("Switched to {}", path.display())));
        return;
    }

    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let document = create_and_activate_document(state, Some(path.clone()), text);
            activate_document(state, document.id());
            state
                .status_message
                .set(Some(format!("Opened {}", path.display())));
        }
        Err(err) => {
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

fn request_close_document(state: &AppState, document_id: DocumentId) {
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

fn finish_pending_action(action: PendingAction, state: &AppState) {
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

fn request_save_as(state: AppState, document: DocumentState) {
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

fn request_save(state: &AppState, document: &DocumentState) {
    if let Some(path) = document.file_path.get_untracked() {
        save_document_to_path(state, document, &path);
    } else {
        request_save_as(state.clone(), document.clone());
    }
}

fn request_open(state: AppState) {
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

        open_document_in_tab(&state, path);
    });
}

fn invoke_command(command_id: &str, state: &AppState) {
    match command_id {
        command_ids::FILE_NEW => create_new_tab(state),
        command_ids::FILE_OPEN => request_open(state.clone()),
        command_ids::FILE_SAVE => {
            let Some(document) = state.active_document_untracked() else {
                state
                    .status_message
                    .set(Some("No active document".to_string()));
                return;
            };
            request_save(state, &document);
        }
        command_ids::FILE_SAVE_AS => {
            let Some(document) = state.active_document_untracked() else {
                state
                    .status_message
                    .set(Some("No active document".to_string()));
                return;
            };
            request_save_as(state.clone(), document);
        }
        _ => state
            .status_message
            .set(Some(format!("Unknown command `{command_id}`"))),
    }
}

fn command_title(command_registry: &CommandRegistry, command_id: &'static str) -> &'static str {
    command_registry
        .get(command_id)
        .map(|command| command.title)
        .unwrap_or(command_id)
}

fn command_menu_entry(
    command_registry: &CommandRegistry,
    command_id: &'static str,
) -> AppMenuEntry {
    let title = command_title(command_registry, command_id).to_string();
    AppMenuEntry::item(title, move |state| invoke_command(command_id, state))
}

fn file_menu_model(command_registry: &CommandRegistry) -> AppMenuModel {
    AppMenuModel::new(
        "file",
        "File",
        vec![
            command_menu_entry(command_registry, command_ids::FILE_NEW),
            command_menu_entry(command_registry, command_ids::FILE_OPEN),
            AppMenuEntry::Separator,
            command_menu_entry(command_registry, command_ids::FILE_SAVE),
            command_menu_entry(command_registry, command_ids::FILE_SAVE_AS),
        ],
    )
}

fn recent_menu_model() -> AppMenuModel {
    AppMenuModel::new(
        "recent",
        "Recent",
        vec![AppMenuEntry::disabled("No recent files yet")],
    )
}

fn app_menu_models(command_registry: &CommandRegistry) -> Vec<AppMenuModel> {
    vec![file_menu_model(command_registry), recent_menu_model()]
}

fn build_menu_entry(menu: Menu, entry: &AppMenuEntry, state: AppState) -> Menu {
    match entry {
        AppMenuEntry::Separator => menu.separator(),
        AppMenuEntry::Item(item) => {
            let title = item.title.clone();
            let enabled = item.enabled;
            let action = item.action.clone();
            menu.item(title, move |menu_item| {
                let menu_item = menu_item.enabled(enabled);
                let Some(action) = action.clone() else {
                    return menu_item;
                };
                let click_state = state.clone();
                menu_item.action(move || action(&click_state))
            })
        }
    }
}

fn build_menu_from_model(menu_model: &AppMenuModel, state: AppState) -> Menu {
    let mut menu = Menu::new();
    for entry in &menu_model.entries {
        menu = build_menu_entry(menu, entry, state.clone());
    }
    menu
}

fn menu_button(menu_model: AppMenuModel, state: AppState) -> impl IntoView {
    Label::new(menu_model.title.clone())
        .style(|s| {
            s.selectable(false)
                .padding_horiz(10.0)
                .padding_vert(6.0)
                .border(1.0)
                .border_color(Color::from_rgb8(196, 199, 204))
                .border_radius(6.0)
                .background(Color::from_rgb8(248, 249, 250))
                .hover(|s| s.background(Color::from_rgb8(232, 236, 240)))
                .active(|s| s.background(Color::from_rgb8(218, 224, 230)))
        })
        .popout_menu(move || build_menu_from_model(&menu_model, state.clone()))
}

fn menu_bar_view(command_registry: CommandRegistry, state: AppState) -> impl IntoView {
    dyn_stack(
        move || app_menu_models(&command_registry),
        |menu| menu.id,
        move |menu| menu_button(menu, state.clone()),
    )
    .style(|s| s.flex_row().col_gap(6.0))
}

fn supported_modifiers(modifiers: Modifiers) -> Modifiers {
    modifiers & (Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::META)
}

fn matches_shortcut(shortcut: Shortcut, keypress: &KeypressKey) -> bool {
    supported_modifiers(keypress.modifiers) == shortcut_modifiers(shortcut.modifiers)
        && shortcut_key_matches(shortcut.key, &keypress.key)
}

fn shortcut_modifiers(modifiers: &[ShortcutModifier]) -> Modifiers {
    let mut resolved = Modifiers::empty();

    for modifier in modifiers {
        match modifier {
            ShortcutModifier::Control => resolved |= Modifiers::CONTROL,
            ShortcutModifier::Alt => resolved |= Modifiers::ALT,
            ShortcutModifier::Shift => resolved |= Modifiers::SHIFT,
            ShortcutModifier::Meta => resolved |= Modifiers::META,
        }
    }

    resolved
}

fn shortcut_key_matches(shortcut_key: ShortcutKey, actual_key: &Key) -> bool {
    match (shortcut_key, actual_key) {
        (ShortcutKey::Character(expected), Key::Character(actual)) => {
            actual.eq_ignore_ascii_case(expected)
        }
        (ShortcutKey::Named("Enter"), Key::Named(NamedKey::Enter)) => true,
        (ShortcutKey::Named("Tab"), Key::Named(NamedKey::Tab)) => true,
        (ShortcutKey::Named("Escape"), Key::Named(NamedKey::Escape)) => true,
        _ => false,
    }
}

fn resolve_shortcut_command(
    command_registry: &CommandRegistry,
    keypress: &KeypressKey,
) -> Option<&'static str> {
    command_registry.iter().find_map(|command| {
        command
            .default_shortcut
            .filter(|shortcut| matches_shortcut(*shortcut, keypress))
            .map(|_| command.id)
    })
}

fn document_editor_view(
    document: DocumentState,
    command_registry: CommandRegistry,
    state: AppState,
) -> impl IntoView {
    let editor_sig = RwSignal::new(document.editor.clone());
    let keymap = KeypressMap::default();

    editor_container_view(
        editor_sig,
        |_| true,
        move |keypress| {
            if let Some(command_id) = resolve_shortcut_command(&command_registry, &keypress) {
                invoke_command(command_id, &state);
                return CommandExecuted::Yes;
            }

            keymap.handle_keypress(editor_sig, &keypress)
        },
    )
    .style(|s| {
        s.apply(editor_theme_style())
            .width_full()
            .min_size(0, 0)
            .flex_grow(1.0)
            .border(1.0)
            .border_color(Color::from_rgb8(220, 223, 227))
    })
}

fn tab_header_view(document: DocumentState, state: AppState) -> impl IntoView {
    let document_id = document.id();
    let activate_state = state.clone();
    let close_state = state.clone();

    Stack::horizontal((
        Label::derived(move || document_title_text(&document))
            .style(|s| s.font_size(12.0).color(Color::from_rgb8(44, 50, 63))),
        Button::new("x")
            .action(move || request_close_document(&close_state, document_id))
            .style(|s| {
                s.padding_horiz(6.0)
                    .padding_vert(2.0)
                    .font_size(11.0)
                    .border(0.0)
                    .border_radius(4.0)
                    .background(Color::from_rgba8(0, 0, 0, 0))
                    .hover(|s| s.background(Color::from_rgb8(226, 229, 233)))
            }),
    ))
    .style(move |s| {
        let is_active = state.documents.get().active_document_id() == Some(document_id);
        s.items_center()
            .col_gap(6.0)
            .padding_horiz(10.0)
            .padding_vert(6.0)
            .border(1.0)
            .border_color(if is_active {
                Color::from_rgb8(199, 205, 214)
            } else {
                Color::from_rgb8(216, 221, 228)
            })
            .border_radius(6.0)
            .background(if is_active {
                Color::from_rgb8(252, 252, 253)
            } else {
                Color::from_rgb8(239, 235, 225)
            })
    })
    .on_event_stop(listener::Click, move |_cx, _| {
        activate_document(&activate_state, document_id);
    })
}

fn tab_strip_view(state: AppState) -> impl IntoView {
    let tab_list_state = state.clone();
    let tab_item_state = state;

    dyn_stack(
        move || tab_list_state.documents(),
        DocumentState::id,
        move |document| tab_header_view(document, tab_item_state.clone()),
    )
    .style(|s| {
        s.flex_row()
            .width_full()
            .padding_horiz(10.0)
            .padding_vert(8.0)
            .col_gap(6.0)
            .background(Color::from_rgb8(241, 237, 228))
    })
    .scroll()
    .style(|s| s.width_full().height(46.0))
}

fn tab_content_view(state: AppState, command_registry: CommandRegistry) -> impl IntoView {
    let active_state = state.clone();
    let documents_state = state.clone();
    let content_state = state;

    tab(
        move || active_state.active_index(),
        move || documents_state.documents(),
        DocumentState::id,
        move |document| {
            document_editor_view(document, command_registry.clone(), content_state.clone())
        },
    )
    .style(|s| s.width_full().min_size(0, 0).flex_grow(1.0))
}

fn confirm_overlay(state: AppState) -> Overlay {
    let backdrop = Empty::new()
        .style(|s| {
            s.absolute()
                .inset(0.0)
                .background(palette::css::BLACK)
                .opacity(0.25)
                .z_index(1)
        })
        .on_event_cont(listener::Click, move |_, _| {
            state.pending_action.set(None);
            state.show_confirm.set(false);
        });

    let save_button = {
        let state = state.clone();
        Button::new("Save").action(move || {
            invoke_command(command_ids::FILE_SAVE, &state);
        })
    };

    let dont_save_button = {
        let state = state.clone();
        Button::new("Don't Save").action(move || {
            if let Some(action) = state.pending_action.get_untracked() {
                finish_pending_action(action, &state);
            } else {
                state.show_confirm.set(false);
            }
        })
    };

    let cancel_button = {
        let state = state.clone();
        Button::new("Cancel").action(move || {
            state.pending_action.set(None);
            state.show_confirm.set(false);
        })
    };

    let buttons =
        Stack::horizontal((save_button, dont_save_button, cancel_button)).style(|s| s.col_gap(8.0));

    let title = {
        let state = state.clone();
        Label::derived(move || match state.pending_action.get() {
            Some(PendingAction::CloseDocument { .. }) => "Unsaved changes".to_string(),
            Some(PendingAction::CloseWindow { .. }) => "Unsaved changes".to_string(),
            None => "Unsaved changes".to_string(),
        })
    }
    .style(|s| s.font_size(18.0).font_bold());

    let message = {
        let state = state.clone();
        Label::derived(move || match state.pending_action.get() {
            Some(PendingAction::CloseDocument { .. }) => {
                "Save your changes before closing this tab?".to_string()
            }
            Some(PendingAction::CloseWindow { .. }) => {
                "Save your changes before closing this window?".to_string()
            }
            None => "Save your changes before continuing?".to_string(),
        })
    }
    .style(|s| s.color(Color::from_rgb8(82, 89, 102)));

    let target_path = {
        let state = state.clone();
        Label::derived(move || {
            let Some(document) = state.active_document() else {
                return String::new();
            };
            document
                .file_path
                .try_get()
                .flatten()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "Untitled".to_string())
        })
    }
    .style({
        let state = state.clone();
        move |s| {
            s.width_full()
                .max_width_full()
                .padding(10.0)
                .font_size(12.0)
                .text_wrap()
                .color(Color::from_rgb8(59, 70, 91))
                .background(Color::from_rgb8(244, 246, 250))
                .border(1.0)
                .border_color(Color::from_rgb8(228, 232, 237))
                .border_radius(8.0)
                .apply_if(state.pending_action.get().is_none(), |s| s.hide())
        }
    });

    let dialog = Stack::vertical((title, message, target_path, buttons)).style(|s| {
        s.absolute()
            .inset_left(40.0)
            .inset_top(40.0)
            .width(420.0)
            .padding(16.0)
            .row_gap(12.0)
            .border(1.0)
            .border_radius(12.0)
            .border_color(Color::from_rgb8(224, 228, 233))
            .background(palette::css::WHITE)
            .z_index(10)
    });

    Overlay::new({
        let state = state.clone();
        Stack::new((backdrop, dialog)).style(move |s| {
            s.fixed()
                .inset(0.0)
                .width_full()
                .height_full()
                .apply_if(!state.show_confirm.get(), |s| s.hide())
        })
    })
}

fn app_view(window_id: WindowId, bootstrap: AppBootstrap) -> impl IntoView {
    let document_scope = Scope::current();
    let documents = RwSignal::new(DocumentSet::empty());
    let status_message = RwSignal::new(None::<String>);
    let pending_action = RwSignal::new(None::<PendingAction>);
    let show_confirm = RwSignal::new(false);
    let save_as_dialog_open = RwSignal::new(false);
    let state = AppState {
        document_scope,
        documents,
        status_message,
        pending_action,
        show_confirm,
        save_as_dialog_open,
    };

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
