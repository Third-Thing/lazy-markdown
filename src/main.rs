use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use floem::{
    close_window, open_file,
    peniko::{color::palette, Color},
    prelude::*,
    save_as,
    views::{
        editor::{
            command::CommandExecuted,
            keypress::{KeypressKey, KeypressMap},
            text::{default_light_theme, Document, SimpleStyling},
            text_document::TextDocument,
            Editor,
        },
        text_editor::text_editor_keys,
        Button, Empty, Label, Overlay, Stack,
    },
    window::{WindowConfig, WindowId},
    Application, FileDialogOptions,
};

#[derive(Clone)]
enum PendingAction {
    CloseWindow(WindowId),
    NewDocument,
    OpenFile(PathBuf),
}

fn current_name(path: Option<&Path>) -> String {
    path.and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn save_editor_to_path(editor: &Editor, path: &Path) -> Result<(), String> {
    std::fs::write(path, editor.doc().text().to_string())
        .map_err(|err| format!("Failed to save {}: {err}", path.display()))?;
    editor.doc().mark_pristine();
    Ok(())
}

fn replace_with_file(
    editor: &Editor,
    file_path: RwSignal<Option<PathBuf>>,
    status_message: RwSignal<Option<String>>,
    path: PathBuf,
) {
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let doc: Rc<dyn Document> = Rc::new(TextDocument::new(editor.cx.get(), text));
            editor.update_doc(doc, None);
            file_path.set(Some(path.clone()));
            status_message.set(Some(format!("Opened {}", path.display())));
        }
        Err(err) => {
            status_message.set(Some(format!("Failed to open {}: {err}", path.display())));
        }
    }
}

fn replace_with_new_document(
    editor: &Editor,
    file_path: RwSignal<Option<PathBuf>>,
    status_message: RwSignal<Option<String>>,
) {
    let doc: Rc<dyn Document> = Rc::new(TextDocument::new(editor.cx.get(), String::new()));
    editor.update_doc(doc, None);
    file_path.set(None);
    status_message.set(Some("Started a new document".to_string()));
}

fn finish_pending_action(
    action: PendingAction,
    editor: &Editor,
    file_path: RwSignal<Option<PathBuf>>,
    status_message: RwSignal<Option<String>>,
    pending_action: RwSignal<Option<PendingAction>>,
    show_confirm: RwSignal<bool>,
) {
    pending_action.set(None);
    show_confirm.set(false);

    match action {
        PendingAction::CloseWindow(window_id) => close_window(window_id),
        PendingAction::NewDocument => {
            replace_with_new_document(editor, file_path, status_message);
        }
        PendingAction::OpenFile(path) => {
            replace_with_file(editor, file_path, status_message, path);
        }
    }
}

fn request_save_as(
    editor: Editor,
    file_path: RwSignal<Option<PathBuf>>,
    status_message: RwSignal<Option<String>>,
    pending_action: RwSignal<Option<PendingAction>>,
    show_confirm: RwSignal<bool>,
) {
    let mut options = FileDialogOptions::new()
        .title("Save file")
        .default_name(current_name(file_path.get_untracked().as_deref()));
    if let Some(path) = file_path
        .get_untracked()
        .as_ref()
        .and_then(|path| path.parent())
    {
        options = options.force_starting_directory(path);
    }

    save_as(options, move |file_info| {
        let Some(path) = file_info.and_then(|info| info.path.into_iter().next()) else {
            return;
        };

        match save_editor_to_path(&editor, &path) {
            Ok(()) => {
                file_path.set(Some(path.clone()));
                status_message.set(Some(format!("Saved {}", path.display())));
                if let Some(action) = pending_action.get_untracked() {
                    finish_pending_action(
                        action,
                        &editor,
                        file_path,
                        status_message,
                        pending_action,
                        show_confirm,
                    );
                }
            }
            Err(err) => status_message.set(Some(err)),
        }
    });
}

fn request_save(
    editor: &Editor,
    file_path: RwSignal<Option<PathBuf>>,
    status_message: RwSignal<Option<String>>,
    pending_action: RwSignal<Option<PendingAction>>,
    show_confirm: RwSignal<bool>,
) {
    if let Some(path) = file_path.get_untracked() {
        match save_editor_to_path(editor, &path) {
            Ok(()) => {
                status_message.set(Some(format!("Saved {}", path.display())));
                if let Some(action) = pending_action.get_untracked() {
                    finish_pending_action(
                        action,
                        editor,
                        file_path,
                        status_message,
                        pending_action,
                        show_confirm,
                    );
                }
            }
            Err(err) => status_message.set(Some(err)),
        }
    } else {
        request_save_as(
            editor.clone(),
            file_path,
            status_message,
            pending_action,
            show_confirm,
        );
    }
}

fn request_open(
    editor: Editor,
    file_path: RwSignal<Option<PathBuf>>,
    status_message: RwSignal<Option<String>>,
    pending_action: RwSignal<Option<PendingAction>>,
    show_confirm: RwSignal<bool>,
) {
    let mut options = FileDialogOptions::new().title("Open file");
    if let Some(path) = file_path
        .get_untracked()
        .as_ref()
        .and_then(|path| path.parent())
    {
        options = options.force_starting_directory(path);
    }

    open_file(options, move |file_info| {
        let Some(path) = file_info.and_then(|info| info.path.into_iter().next()) else {
            return;
        };

        if editor.doc().is_dirty() {
            pending_action.set(Some(PendingAction::OpenFile(path)));
            show_confirm.set(true);
        } else {
            replace_with_file(&editor, file_path, status_message, path);
        }
    });
}

fn confirm_overlay(
    editor: Editor,
    file_path: RwSignal<Option<PathBuf>>,
    status_message: RwSignal<Option<String>>,
    pending_action: RwSignal<Option<PendingAction>>,
    show_confirm: RwSignal<bool>,
) -> Overlay {
    let backdrop = Empty::new()
        .style(|s| {
            s.absolute()
                .inset(0.0)
                .background(palette::css::BLACK)
                .opacity(0.25)
                .z_index(1)
        })
        .on_event_cont(listener::Click, move |_, _| {
            pending_action.set(None);
            show_confirm.set(false);
        });

    let save_button = {
        let editor = editor.clone();
        Button::new("Save").action(move || {
            request_save(
                &editor,
                file_path,
                status_message,
                pending_action,
                show_confirm,
            );
        })
    };

    let dont_save_button = {
        let editor = editor.clone();
        Button::new("Don't Save").action(move || {
            if let Some(action) = pending_action.get_untracked() {
                finish_pending_action(
                    action,
                    &editor,
                    file_path,
                    status_message,
                    pending_action,
                    show_confirm,
                );
            } else {
                show_confirm.set(false);
            }
        })
    };

    let cancel_button = Button::new("Cancel").action(move || {
        pending_action.set(None);
        show_confirm.set(false);
    });

    let buttons =
        Stack::horizontal((save_button, dont_save_button, cancel_button)).style(|s| s.col_gap(8.0));

    let title = Label::derived(move || match pending_action.get() {
        Some(PendingAction::CloseWindow(_)) => "Unsaved changes".to_string(),
        Some(PendingAction::NewDocument) => "Start a new document?".to_string(),
        Some(PendingAction::OpenFile(_)) => "Open a different file?".to_string(),
        None => "Unsaved changes".to_string(),
    })
    .style(|s| s.font_size(18.0).font_bold());

    let message = Label::derived(move || match pending_action.get() {
        Some(PendingAction::CloseWindow(_)) => {
            "Save your changes before closing this window?".to_string()
        }
        Some(PendingAction::NewDocument) => {
            "Save your changes before starting a new document?".to_string()
        }
        Some(PendingAction::OpenFile(_)) => {
            "Save your changes before opening a different file?".to_string()
        }
        None => "Save your changes before continuing?".to_string(),
    })
    .style(|s| s.color(Color::from_rgb8(82, 89, 102)));

    let target_path = Label::derived(move || match pending_action.get() {
        Some(PendingAction::OpenFile(path)) => path.display().to_string(),
        _ => String::new(),
    })
    .style(move |s| {
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
            .apply_if(
                !matches!(pending_action.get(), Some(PendingAction::OpenFile(_))),
                |s| s.hide(),
            )
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

    Overlay::new(Stack::new((backdrop, dialog)).style(move |s| {
        s.fixed()
            .inset(0.0)
            .width_full()
            .height_full()
            .apply_if(!show_confirm.get(), |s| s.hide())
    }))
}

fn app_view(window_id: WindowId) -> impl IntoView {
    let file_path = RwSignal::new(std::env::args().nth(1).map(PathBuf::from));
    let status_message = RwSignal::new(None::<String>);
    let pending_action = RwSignal::new(None::<PendingAction>);
    let show_confirm = RwSignal::new(false);

    let initial_text = match file_path.get_untracked() {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                status_message.set(Some(format!("Failed to open {}: {err}", path.display())));
                file_path.set(None);
                String::new()
            }
        },
        None => String::new(),
    };

    let file_path_for_keys = file_path;
    let status_for_keys = status_message;
    let pending_for_keys = pending_action;
    let confirm_for_keys = show_confirm;
    let keymap = KeypressMap::default();

    let editor = text_editor_keys(
        initial_text,
        move |editor_sig: RwSignal<Editor>, keypress: &KeypressKey| {
            if keypress.modifiers == Modifiers::CONTROL
                && keypress.key == Key::Character("s".into())
            {
                editor_sig.with_untracked(|editor| {
                    request_save(
                        editor,
                        file_path_for_keys,
                        status_for_keys,
                        pending_for_keys,
                        confirm_for_keys,
                    );
                });
                return CommandExecuted::Yes;
            }

            keymap.handle_keypress(editor_sig, keypress)
        },
    );

    let editor_state = editor.editor().clone();

    let open_button = {
        let editor = editor_state.clone();
        Button::new("Open").action(move || {
            request_open(
                editor.clone(),
                file_path,
                status_message,
                pending_action,
                show_confirm,
            );
        })
    };

    let new_button = {
        let editor = editor_state.clone();
        Button::new("New").action(move || {
            if editor.doc().is_dirty() {
                pending_action.set(Some(PendingAction::NewDocument));
                show_confirm.set(true);
            } else {
                replace_with_new_document(&editor, file_path, status_message);
            }
        })
    };

    let save_button = {
        let editor = editor_state.clone();
        Button::new("Save").action(move || {
            request_save(
                &editor,
                file_path,
                status_message,
                pending_action,
                show_confirm,
            );
        })
    };

    let save_as_button = {
        let editor = editor_state.clone();
        Button::new("Save As").action(move || {
            request_save_as(
                editor.clone(),
                file_path,
                status_message,
                pending_action,
                show_confirm,
            );
        })
    };

    let top_bar_editor = editor_state.clone();
    let top_bar = Stack::horizontal((
        Stack::horizontal((new_button, open_button, save_button, save_as_button))
            .style(|s| s.col_gap(8.0)),
        Label::derived(move || {
            let path = file_path.get();
            let doc = top_bar_editor.doc_track();
            let modified = if doc.dirty().get() { " *" } else { "" };
            format!("{}{}", current_name(path.as_deref()), modified)
        })
        .style(|s| {
            s.font_size(13.0)
                .font_bold()
                .color(Color::from_rgb8(44, 50, 63))
        }),
    ))
    .style(|s| {
        s.width_full()
            .justify_between()
            .items_center()
            .padding_horiz(10.0)
            .padding_vert(9.0)
            .background(Color::from_rgb8(236, 232, 221))
    });

    let status_strip =
        Label::derived(move || status_message.get().unwrap_or_default()).style(move |s| {
            s.width_full()
                .padding_horiz(12.0)
                .padding_vert(8.0)
                .font_size(12.0)
                .color(Color::from_rgb8(82, 89, 102))
                .background(Color::from_rgb8(243, 239, 230))
                .apply_if(status_message.get().is_none(), |s| s.hide())
        });

    let editor_view = editor
        .styling(SimpleStyling::new())
        .editor_style(default_light_theme)
        .style(|s| {
            s.width_full()
                .min_size(0, 0)
                .flex_grow(1.0)
                .border(1.0)
                .border_color(Color::from_rgb8(220, 223, 227))
        });

    let title_editor = editor_state.clone();
    let close_editor = editor_state.clone();

    let root = Stack::new((
        Stack::vertical((top_bar, status_strip, editor_view)).style(|s| {
            s.size_full()
                .padding(10.0)
                .row_gap(0.0)
                .background(Color::from_rgb8(247, 243, 233))
        }),
        confirm_overlay(
            editor_state.clone(),
            file_path,
            status_message,
            pending_action,
            show_confirm,
        ),
    ))
    .style(|s| s.size_full())
    .window_title(move || {
        let path = file_path.get();
        let doc = title_editor.doc_track();
        let modified = if doc.dirty().get() { " *" } else { "" };
        format!("{}{}", current_name(path.as_deref()), modified)
    })
    .on_event_cont(listener::WindowCloseRequested, move |cx, _| {
        if close_editor.doc().is_dirty() {
            cx.prevent_default();
            pending_action.set(Some(PendingAction::CloseWindow(window_id)));
            show_confirm.set(true);
        }
    });

    root
}

fn main() {
    Application::new()
        .window(
            app_view,
            Some(
                WindowConfig::default()
                    .size((920.0, 680.0))
                    .min_size((480.0, 320.0))
                    .title("lazy-markdown"),
            ),
        )
        .run();
}
