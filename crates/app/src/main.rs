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
    save_as,
    views::{
        Button, Empty, Label, Overlay, Stack,
        editor::{
            Editor,
            command::CommandExecuted,
            keypress::{KeypressKey, KeypressMap},
            text::{Document, SimpleStyling, default_light_theme},
            text_document::TextDocument,
        },
        text_editor::text_editor_keys,
    },
    window::{WindowConfig, WindowId},
};
use lazy_markdown_core::{
    CommandRegistry, ModuleRegistry, Shortcut, ShortcutKey, ShortcutModifier, command_ids,
    register_builtin_commands,
};

#[derive(Clone)]
enum PendingAction {
    CloseWindow(WindowId),
    NewDocument,
    OpenFile(PathBuf),
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
struct AppState {
    file_path: RwSignal<Option<PathBuf>>,
    status_message: RwSignal<Option<String>>,
    pending_action: RwSignal<Option<PendingAction>>,
    show_confirm: RwSignal<bool>,
    save_as_dialog_open: RwSignal<bool>,
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

fn save_editor_to_path(state: &AppState, editor: &Editor, path: &Path) {
    match write_editor_text(editor, path) {
        Ok(()) => {
            editor.doc().mark_pristine();
            state.file_path.set(Some(path.to_path_buf()));
            state
                .status_message
                .set(Some(format!("Saved {}", path.display())));
            if let Some(action) = state.pending_action.get_untracked() {
                finish_pending_action(action, state, editor);
            }
        }
        Err(err) => state.status_message.set(Some(err)),
    }
}

fn replace_with_file(state: &AppState, editor: &Editor, path: PathBuf) {
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let doc: Rc<dyn Document> = Rc::new(TextDocument::new(editor.cx.get(), text));
            editor.update_doc(doc, None);
            state.file_path.set(Some(path.clone()));
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

fn replace_with_new_document(state: &AppState, editor: &Editor) {
    let doc: Rc<dyn Document> = Rc::new(TextDocument::new(editor.cx.get(), String::new()));
    editor.update_doc(doc, None);
    state.file_path.set(None);
    state
        .status_message
        .set(Some("Started a new document".to_string()));
}

fn finish_pending_action(action: PendingAction, state: &AppState, editor: &Editor) {
    state.pending_action.set(None);
    state.show_confirm.set(false);

    match action {
        PendingAction::CloseWindow(window_id) => close_window(window_id),
        PendingAction::NewDocument => replace_with_new_document(state, editor),
        PendingAction::OpenFile(path) => replace_with_file(state, editor, path),
    }
}

fn request_save_as(state: AppState, editor: Editor) {
    if state.save_as_dialog_open.get_untracked() {
        return;
    }

    state.save_as_dialog_open.set(true);

    let mut options = FileDialogOptions::new()
        .title("Save file")
        .default_name(current_name(state.file_path.get_untracked().as_deref()));

    if let Some(path) = state
        .file_path
        .get_untracked()
        .as_ref()
        .and_then(|path| path.parent())
    {
        options = options.force_starting_directory(path);
    }

    save_as(options, move |file_info| {
        let Some(path) = file_info.and_then(|info| info.path.into_iter().next()) else {
            state.save_as_dialog_open.set(false);
            return;
        };

        save_editor_to_path(&state, &editor, &path);
        state.save_as_dialog_open.set(false);
    });
}

fn request_save(state: &AppState, editor: &Editor) {
    if let Some(path) = state.file_path.get_untracked() {
        save_editor_to_path(state, editor, &path);
    } else {
        request_save_as(state.clone(), editor.clone());
    }
}

fn request_open(state: AppState, editor: Editor) {
    let mut options = FileDialogOptions::new().title("Open file");
    if let Some(path) = state
        .file_path
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
            state
                .pending_action
                .set(Some(PendingAction::OpenFile(path)));
            state.show_confirm.set(true);
        } else {
            replace_with_file(&state, &editor, path);
        }
    });
}

fn invoke_command(command_id: &str, state: &AppState, editor: &Editor) {
    match command_id {
        command_ids::FILE_NEW => {
            if editor.doc().is_dirty() {
                state.pending_action.set(Some(PendingAction::NewDocument));
                state.show_confirm.set(true);
            } else {
                replace_with_new_document(state, editor);
            }
        }
        command_ids::FILE_OPEN => request_open(state.clone(), editor.clone()),
        command_ids::FILE_SAVE => request_save(state, editor),
        command_ids::FILE_SAVE_AS => request_save_as(state.clone(), editor.clone()),
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

fn command_menu(command_registry: &CommandRegistry, state: AppState, editor: Editor) -> Menu {
    let new_title = command_title(command_registry, command_ids::FILE_NEW);
    let open_title = command_title(command_registry, command_ids::FILE_OPEN);
    let save_title = command_title(command_registry, command_ids::FILE_SAVE);
    let save_as_title = command_title(command_registry, command_ids::FILE_SAVE_AS);

    let new_state = state.clone();
    let new_editor = editor.clone();
    let open_state = state.clone();
    let open_editor = editor.clone();
    let save_state = state.clone();
    let save_editor = editor.clone();
    let save_as_state = state;
    let save_as_editor = editor;

    Menu::new()
        .item(new_title, move |item| {
            item.action(move || invoke_command(command_ids::FILE_NEW, &new_state, &new_editor))
        })
        .item(open_title, move |item| {
            item.action(move || invoke_command(command_ids::FILE_OPEN, &open_state, &open_editor))
        })
        .separator()
        .item(save_title, move |item| {
            item.action(move || invoke_command(command_ids::FILE_SAVE, &save_state, &save_editor))
        })
        .item(save_as_title, move |item| {
            item.action(move || {
                invoke_command(command_ids::FILE_SAVE_AS, &save_as_state, &save_as_editor)
            })
        })
}

fn menu_button(
    command_registry: CommandRegistry,
    state: AppState,
    editor: Editor,
) -> impl IntoView {
    Label::new("File")
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
        .popout_menu(move || command_menu(&command_registry, state.clone(), editor.clone()))
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

fn confirm_overlay(state: AppState, editor: Editor) -> Overlay {
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
        let editor = editor.clone();
        Button::new("Save").action(move || {
            invoke_command(command_ids::FILE_SAVE, &state, &editor);
        })
    };

    let dont_save_button = {
        let state = state.clone();
        let editor = editor.clone();
        Button::new("Don't Save").action(move || {
            if let Some(action) = state.pending_action.get_untracked() {
                finish_pending_action(action, &state, &editor);
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
            Some(PendingAction::CloseWindow(_)) => "Unsaved changes".to_string(),
            Some(PendingAction::NewDocument) => "Start a new document?".to_string(),
            Some(PendingAction::OpenFile(_)) => "Open a different file?".to_string(),
            None => "Unsaved changes".to_string(),
        })
    }
    .style(|s| s.font_size(18.0).font_bold());

    let message = {
        let state = state.clone();
        Label::derived(move || match state.pending_action.get() {
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
    }
    .style(|s| s.color(Color::from_rgb8(82, 89, 102)));

    let target_path = {
        let state = state.clone();
        Label::derived(move || match state.pending_action.get() {
            Some(PendingAction::OpenFile(path)) => path.display().to_string(),
            _ => String::new(),
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
                .apply_if(
                    !matches!(state.pending_action.get(), Some(PendingAction::OpenFile(_))),
                    |s| s.hide(),
                )
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
    let file_path = RwSignal::new(std::env::args().nth(1).map(PathBuf::from));
    let status_message = RwSignal::new(None::<String>);
    let pending_action = RwSignal::new(None::<PendingAction>);
    let show_confirm = RwSignal::new(false);
    let save_as_dialog_open = RwSignal::new(false);
    let state = AppState {
        file_path,
        status_message,
        pending_action,
        show_confirm,
        save_as_dialog_open,
    };

    let initial_text = match file_path.get_untracked() {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                state
                    .status_message
                    .set(Some(format!("Failed to open {}: {err}", path.display())));
                state.file_path.set(None);
                String::new()
            }
        },
        None => String::new(),
    };

    let command_registry_for_keys = bootstrap.command_registry.clone();
    let state_for_keys = state.clone();
    let keymap = KeypressMap::default();

    let editor = text_editor_keys(
        initial_text,
        move |editor_sig: RwSignal<Editor>, keypress| {
            if let Some(command_id) = resolve_shortcut_command(&command_registry_for_keys, keypress)
            {
                editor_sig.with_untracked(|editor| {
                    invoke_command(command_id, &state_for_keys, editor);
                });
                return CommandExecuted::Yes;
            }

            keymap.handle_keypress(editor_sig, keypress)
        },
    );

    let editor_state = editor.editor().clone();

    let main_menu = menu_button(
        bootstrap.command_registry.clone(),
        state.clone(),
        editor_state.clone(),
    );

    let top_bar_editor = editor_state.clone();
    let top_bar = {
        let state = state.clone();
        Stack::horizontal((
            main_menu,
            Label::derived(move || {
                let path = state.file_path.get();
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

    Stack::new((
        Stack::vertical((top_bar, status_strip, editor_view)).style(|s| {
            s.size_full()
                .padding(10.0)
                .row_gap(0.0)
                .background(Color::from_rgb8(247, 243, 233))
        }),
        confirm_overlay(state.clone(), editor_state),
    ))
    .style(|s| s.size_full())
    .window_title({
        let state = state.clone();
        move || {
            let path = state.file_path.get();
            let doc = title_editor.doc_track();
            let modified = if doc.dirty().get() { " *" } else { "" };
            format!("{}{}", current_name(path.as_deref()), modified)
        }
    })
    .on_event_cont(listener::WindowCloseRequested, {
        let state = state.clone();
        move |cx, _| {
            if close_editor.doc().is_dirty() {
                cx.prevent_default();
                state
                    .pending_action
                    .set(Some(PendingAction::CloseWindow(window_id)));
                state.show_confirm.set(true);
            }
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
