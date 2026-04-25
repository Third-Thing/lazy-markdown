use std::path::{Path, PathBuf};

use gpui::{
    App, AppContext as _, Context, Entity, Focusable as _, InteractiveElement as _, IntoElement,
    KeyBinding, Menu, MenuItem, ParentElement as _, PathPromptOptions, Render, SharedString,
    Styled as _, Subscription, Window, WindowBounds, WindowOptions, actions, div, px, size,
};
use gpui_component::{
    GlobalState, Root, WindowExt,
    button::ButtonVariant,
    dialog::DialogButtonProps,
    input::{Input, InputEvent, InputState},
    menu::AppMenuBar,
    v_flex,
};
use gpui_component_assets::Assets;

actions!(probe, [New, Open, Save, SaveAs]);

const SAMPLE_MARKDOWN: &str = r#"# lazy-markdown GPUI probe

This standalone crate opens a GPUI Component code editor in Markdown mode.

- Type here
- Try selection and paste
- Use this to validate editor behavior before converting the main app
"#;

#[derive(Clone, Copy)]
enum PendingDocumentAction {
    New,
    Open,
}

struct ProbeWindow {
    editor: Entity<InputState>,
    app_menu_bar: Entity<AppMenuBar>,
    current_path: Option<PathBuf>,
    pristine_text: String,
    dirty: bool,
    status: SharedString,
    _subscriptions: Vec<Subscription>,
}

impl ProbeWindow {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        install_app_menus(cx);
        let app_menu_bar = AppMenuBar::new(cx);
        app_menu_bar.update(cx, |menu_bar, cx| menu_bar.reload(cx));

        let editor = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .searchable(true)
                .default_value(SAMPLE_MARKDOWN)
        });

        let focus_handle = editor.focus_handle(cx);
        window.defer(cx, move |window, cx| {
            focus_handle.focus(window, cx);
        });

        let _subscriptions = vec![cx.subscribe_in(&editor, window, {
            let editor = editor.clone();
            move |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = editor.read(cx).value();
                    this.dirty = value.as_ref() != this.pristine_text;
                    this.status = document_status(&value, this.dirty);
                    cx.notify();
                }
            }
        })];

        let value = editor.read(cx).value();
        let status = format!(
            "{} lines, {} chars",
            value.lines().count(),
            value.chars().count()
        )
        .into();

        Self {
            editor,
            app_menu_bar,
            current_path: None,
            pristine_text: SAMPLE_MARKDOWN.to_string(),
            dirty: false,
            status,
            _subscriptions,
        }
    }

    fn on_new(&mut self, _: &New, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_or_run(PendingDocumentAction::New, window, cx);
    }

    fn on_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_or_run(PendingDocumentAction::Open, window, cx);
    }

    fn on_save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.current_path.clone() {
            self.save_to_path(&path, cx);
        } else {
            self.prompt_save_as(window, cx);
        }
    }

    fn on_save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt_save_as(window, cx);
    }

    fn confirm_or_run(
        &mut self,
        action: PendingDocumentAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dirty {
            self.open_discard_changes_dialog(action, window, cx);
        } else {
            self.run_document_action(action, window, cx);
        }
    }

    fn open_discard_changes_dialog(
        &mut self,
        action: PendingDocumentAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.entity();
        let document_name = self
            .current_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        let description =
            format!("{document_name} has unsaved changes. Discard them and continue?");

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
                            this.run_document_action(action, window, cx);
                        });
                    });

                    true
                })
        });
    }

    fn should_close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.dirty {
            return true;
        }

        if !window.has_active_dialog(cx) {
            self.open_close_window_dialog(window, cx);
        }

        false
    }

    fn open_close_window_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let document_name = self
            .current_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
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

    fn run_document_action(
        &mut self,
        action: PendingDocumentAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            PendingDocumentAction::New => self.new_document(window, cx),
            PendingDocumentAction::Open => self.prompt_open_document(window, cx),
        }
    }

    fn new_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |editor, cx| {
            editor.set_value("", window, cx);
        });
        self.current_path = None;
        self.pristine_text.clear();
        self.dirty = false;
        self.status = "Started a new document".into();
        cx.notify();
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
                _ = view.update(cx, |this, cx| {
                    match contents {
                        Ok(contents) => {
                            this.editor.update(cx, |editor, cx| {
                                editor.set_value(contents.clone(), window, cx);
                            });
                            this.current_path = Some(path.clone());
                            this.pristine_text = contents;
                            this.dirty = false;
                            this.status = format!("Opened {}", path.display()).into();
                        }
                        Err(err) => {
                            this.status =
                                format!("Failed to open {}: {err}", path.display()).into();
                        }
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn save_to_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        let contents = self.editor.read(cx).value();
        match std::fs::write(path, contents.as_ref()) {
            Ok(()) => {
                self.current_path = Some(path.to_path_buf());
                self.pristine_text = contents.to_string();
                self.dirty = false;
                self.status = format!("Saved {}", path.display()).into();
            }
            Err(err) => {
                self.status = format!("Failed to save {}: {err}", path.display()).into();
            }
        }
        cx.notify();
    }

    fn prompt_save_as(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let directory = self
            .current_path
            .as_ref()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let suggested_name = self
            .current_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("untitled.md")
            .to_string();
        let contents = self.editor.read(cx).value().to_string();
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
                            this.current_path = Some(path.clone());
                            this.pristine_text = contents.clone();
                            this.dirty = false;
                            this.status = format!("Saved {}", path.display()).into();
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
}

impl Render for ProbeWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);

        v_flex()
            .id("gpui-editor-probe")
            .size_full()
            .relative()
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
                    .child(self.app_menu_bar.clone()),
            )
            .gap_2()
            .p_3()
            .child(Input::new(&self.editor).size_full())
            .child(div().text_sm().child(self.status.clone()))
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
