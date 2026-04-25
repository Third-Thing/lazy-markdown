use std::path::{Path, PathBuf};

use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _,
    IntoElement, KeyBinding, Menu, MenuItem, ParentElement as _, PathPromptOptions, Render,
    SharedString, Styled as _, Subscription, Window, WindowBounds, WindowOptions, actions, div, px,
    size,
};
use gpui_component::{
    ActiveTheme as _, GlobalState, IconName, Root, Sizable as _, WindowExt,
    button::{Button, ButtonVariant, ButtonVariants as _},
    dialog::DialogButtonProps,
    input::{Input, InputEvent, InputState},
    menu::AppMenuBar,
    tab::{Tab, TabBar},
    v_flex,
};
use gpui_component_assets::Assets;

actions!(probe, [New, Open, Save, SaveAs]);

const MAX_OPEN_TABS: usize = 5;

const SAMPLE_MARKDOWN: &str = r#"# lazy-markdown GPUI probe

This standalone crate opens GPUI Component multiline text editors in tabs.

- Type here
- Try selection and paste
- Use this to validate editor behavior before converting the main app
"#;

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

struct ProbeDocument {
    id: DocumentId,
    editor: Entity<InputState>,
    current_path: Option<PathBuf>,
    pristine_text: String,
    dirty: bool,
}

impl ProbeDocument {
    fn title(&self) -> SharedString {
        let marker = if self.dirty { " *" } else { "" };
        format!("{}{marker}", self.saved_title()).into()
    }

    fn saved_title(&self) -> String {
        current_name(self.current_path.as_deref())
    }
}

struct ProbeWindow {
    documents: Vec<ProbeDocument>,
    active_document_id: Option<DocumentId>,
    next_document_id: DocumentId,
    app_menu_bar: Entity<AppMenuBar>,
    status: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl ProbeWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        install_app_menus(cx);
        let app_menu_bar = AppMenuBar::new(cx);
        app_menu_bar.update(cx, |menu_bar, cx| menu_bar.reload(cx));

        let mut this = Self {
            documents: Vec::new(),
            active_document_id: None,
            next_document_id: DocumentId::initial(),
            app_menu_bar,
            status: "Ready".into(),
            _subscriptions: Vec::new(),
        };
        this.create_document(
            None,
            SAMPLE_MARKDOWN.to_string(),
            "Opened the GPUI editor probe".into(),
            window,
            cx,
        );
        this
    }

    fn on_new(&mut self, _: &New, window: &mut Window, cx: &mut Context<Self>) {
        self.create_new_document(window, cx);
    }

    fn on_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt_open_document(window, cx);
    }

    fn on_save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        let Some(document) = self.active_document() else {
            return;
        };
        let document_id = document.id;

        match document.current_path.clone() {
            Some(path) => self.save_to_path(document_id, &path, cx),
            None => self.prompt_save_as(document_id, window, cx),
        }
    }

    fn on_save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(document_id) = self.active_document_id {
            self.prompt_save_as(document_id, window, cx);
        }
    }

    fn allocate_document_id(&mut self) -> DocumentId {
        let id = self.next_document_id;
        self.next_document_id = self.next_document_id.next();
        id
    }

    fn create_document(
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
        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .searchable(true)
                .default_value(text.clone())
        });
        let subscription = cx.subscribe_in(&editor, window, {
            let editor = editor.clone();
            move |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.refresh_document_status(id, &editor, cx);
                }
            }
        });

        self._subscriptions.push(subscription);
        self.documents.push(ProbeDocument {
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

    fn should_close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let dirty_documents = self.dirty_document_ids();
        if dirty_documents.is_empty() {
            return true;
        }

        if !window.has_active_dialog(cx) {
            self.activate_document(dirty_documents[0], window, cx);
            self.open_close_window_dialog(window, cx);
        }

        false
    }

    fn open_close_window_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let document_name = self
            .active_document()
            .map(ProbeDocument::saved_title)
            .unwrap_or_else(|| "Untitled".to_string());
        let description = format!("{document_name} has unsaved changes. Close without saving?");

        window.open_alert_dialog(cx, move |dialog, _, _| {
            dialog
                .title("Discard changes and close?")
                .description(description.clone())
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Discard and Close")
                        .ok_variant(ButtonVariant::Danger)
                        .cancel_text("Keep Editing")
                        .show_cancel(true),
                )
                .on_ok(|_, window, _| {
                    window.remove_window();
                    true
                })
        });
    }

    fn create_new_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.create_document(
            None,
            String::new(),
            "Started a new document".into(),
            window,
            cx,
        );
    }

    fn prompt_open_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn open_document_path(
        &mut self,
        path: PathBuf,
        contents: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(document_id) = self.document_id_for_path(&path) {
            self.activate_document(document_id, window, cx);
            self.status = format!("Switched to {}", path.display()).into();
            cx.notify();
            return;
        }

        self.create_document(
            Some(path.clone()),
            contents,
            format!("Opened {}", path.display()).into(),
            window,
            cx,
        );
    }

    fn save_to_path(&mut self, document_id: DocumentId, path: &Path, cx: &mut Context<Self>) {
        let Some(index) = self.document_index(document_id) else {
            return;
        };
        let contents = self.documents[index].editor.read(cx).value().to_string();
        match std::fs::write(path, contents.as_bytes()) {
            Ok(()) => {
                let document = &mut self.documents[index];
                document.current_path = Some(path.to_path_buf());
                document.pristine_text = contents;
                document.dirty = false;
                self.status = format!("Saved {}", path.display()).into();
            }
            Err(err) => {
                self.status = format!("Failed to save {}: {err}", path.display()).into();
            }
        }
        cx.notify();
    }

    fn prompt_save_as(
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
            let result = std::fs::write(&path, &contents);

            _ = window.update(|_, cx| {
                _ = view.update(cx, |this, cx| {
                    match result {
                        Ok(()) => {
                            if let Some(index) = this.document_index(document_id) {
                                let document = &mut this.documents[index];
                                document.current_path = Some(path.clone());
                                document.pristine_text = contents.clone();
                                document.dirty = false;
                                this.status = format!("Saved {}", path.display()).into();
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

    fn request_close_document(
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
            .map(ProbeDocument::saved_title)
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

    fn activate_document(
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

    fn activate_index(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
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

    fn dirty_document_ids(&self) -> Vec<DocumentId> {
        self.documents
            .iter()
            .filter(|document| document.dirty)
            .map(|document| document.id)
            .collect()
    }

    fn active_index(&self) -> Option<usize> {
        let active_document_id = self.active_document_id?;
        self.document_index(active_document_id)
    }

    fn active_document(&self) -> Option<&ProbeDocument> {
        self.active_document_id
            .and_then(|document_id| self.document_by_id(document_id))
    }

    fn active_editor(&self) -> Option<Entity<InputState>> {
        self.active_document()
            .map(|document| document.editor.clone())
    }

    fn document_by_id(&self, document_id: DocumentId) -> Option<&ProbeDocument> {
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

impl Render for ProbeWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let view = cx.entity();
        let tabs = self.documents.iter().map({
            let view = view.clone();
            move |document| {
                let document_id = document.id;
                let view = view.clone();
                Tab::new().label(document.title()).suffix(
                    Button::new(format!("close-tab-{}", document_id.0))
                        .ghost()
                        .xsmall()
                        .icon(IconName::Close)
                        .tooltip("Close tab")
                        .tab_stop(false)
                        .on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            _ = view.update(cx, |this, cx| {
                                this.request_close_document(document_id, window, cx);
                            });
                        }),
                )
            }
        });
        let active_editor = self.active_editor();
        let selected_index = self.active_index().unwrap_or(0);

        v_flex()
            .id("gpui-editor-probe")
            .size_full()
            .relative()
            .bg(cx.theme().background)
            .on_action(cx.listener(Self::on_new))
            .on_action(cx.listener(Self::on_open))
            .on_action(cx.listener(Self::on_save))
            .on_action(cx.listener(Self::on_save_as))
            .child(
                div()
                    .w_full()
                    .h(px(32.))
                    .px_2()
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(self.app_menu_bar.clone()),
            )
            .child(
                div().w_full().px_3().pt_3().child(
                    TabBar::new("document-tabs")
                        .selected_index(selected_index)
                        .menu(true)
                        .children(tabs)
                        .on_click({
                            let view = view.clone();
                            move |ix, window, cx| {
                                _ = view.update(cx, |this, cx| {
                                    this.activate_index(*ix, window, cx);
                                });
                            }
                        }),
                ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .px_3()
                    .pt_2()
                    .child(match active_editor {
                        Some(editor) => Input::new(&editor).size_full().into_any_element(),
                        None => div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(cx.theme().muted_foreground)
                            .child("No open documents")
                            .into_any_element(),
                    } as AnyElement),
            )
            .child(
                div()
                    .w_full()
                    .px_3()
                    .py_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(self.status.clone()),
            )
            .children(dialog_layer)
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx: &mut App| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(900.), px(650.)), cx)),
            titlebar: None,
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| ProbeWindow::new(window, cx));
                window.on_window_should_close(cx, {
                    let view = view.clone();
                    move |window, cx| {
                        view.update(cx, |this, cx| this.should_close_window(window, cx))
                    }
                });
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("open GPUI editor probe window");
        })
        .detach();
    });
}

fn install_app_menus(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-n", New, None),
        KeyBinding::new("ctrl-o", Open, None),
        KeyBinding::new("ctrl-s", Save, None),
        KeyBinding::new("ctrl-shift-s", SaveAs, None),
    ]);

    let owned_menus = build_app_menus().into_iter().map(Menu::owned).collect();
    cx.set_menus(build_app_menus());
    GlobalState::global_mut(cx).set_app_menus(owned_menus);
}

fn build_app_menus() -> Vec<Menu> {
    vec![Menu {
        name: "File".into(),
        items: vec![
            MenuItem::action("New", New),
            MenuItem::separator(),
            MenuItem::action("Open...", Open),
            MenuItem::separator(),
            MenuItem::action("Save", Save),
            MenuItem::action("Save As...", SaveAs),
        ],
        disabled: false,
    }]
}

fn current_name(path: Option<&Path>) -> String {
    path.and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn document_status(value: &str, dirty: bool) -> SharedString {
    let marker = if dirty { "modified, " } else { "" };
    format!(
        "{}{} lines, {} chars",
        marker,
        value.lines().count(),
        value.chars().count()
    )
    .into()
}
