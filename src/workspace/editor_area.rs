use floem::{
    Clipboard,
    context::VisualChanged,
    event::{EventPropagation, listener},
    kurbo::{Point, Rect, Vec2},
    prelude::*,
    reactive::Effect,
    style::CursorStyle,
    taffy::style::Overflow,
    ui_events::{
        keyboard::{Key, KeyboardEvent, Modifiers},
        pointer::{PointerButton, PointerButtonEvent, PointerState},
    },
    view::ViewId,
    views::{
        Container, Empty, Label, Scroll, Stack,
        editor::{
            Editor,
            command::Command as EditorCommand,
            command::CommandExecuted,
            core::command::EditCommand,
            keypress::{KeypressKey, default_key_handler},
            view::{LineRegion, cursor_caret, editor_gutter, editor_view},
        },
        tab,
    },
};

use crate::preferences::theme::editor_theme_style;

use super::state::{AppState, DocumentId, DocumentState};

const CONTEXT_MENU_CURSOR_GAP: f64 = 4.0;

fn run_editor_command(document: &DocumentState, command: EditorCommand) {
    document
        .editor
        .doc()
        .run_command(&document.editor, &command, Some(1), Modifiers::empty());

    if let Some(view_id) = document.editor.editor_view_id.get_untracked() {
        view_id.request_focus();
    }
}

fn run_editor_clipboard_command(document: &DocumentState, command: EditCommand) {
    run_editor_command(document, EditorCommand::Edit(command));
}

fn editor_has_selection(document: &DocumentState) -> bool {
    let doc = document.editor.doc();
    document
        .editor
        .cursor
        .with_untracked(|cursor| !cursor.edit_selection(&doc.rope_text()).is_caret())
}

fn editor_can_paste() -> bool {
    Clipboard::get_contents().is_ok_and(|content| !content.is_empty())
}

fn move_caret_for_secondary_click(editor: &floem::views::editor::Editor, state: &PointerState) {
    let mode = editor.cursor.with_untracked(|cursor| cursor.get_mode());
    let (offset, ..) = editor.offset_of_point(mode, state.logical_point());
    let doc = editor.doc();
    let pointer_inside_selection = editor
        .cursor
        .with_untracked(|cursor| cursor.edit_selection(&doc.rope_text()).contains(offset));

    if !pointer_inside_selection {
        editor.single_click(state);
    }
}

fn open_editor_context_menu(state: &AppState, document: &DocumentState, position: Point) {
    state.open_editor_context_menu(document.id(), position);
}

fn context_menu_anchor(view_id: ViewId, state: &PointerState) -> Point {
    let window_point = view_id.get_visual_transform() * state.logical_point();
    Point::new(
        window_point.x.round(),
        (window_point.y + CONTEXT_MENU_CURSOR_GAP).round(),
    )
}

fn context_menu_item(
    title: &'static str,
    enabled: bool,
    action: impl Fn() + 'static,
    state: AppState,
) -> impl IntoView {
    Label::new(title)
        .style(move |s| {
            let theme = state.app_theme();
            s.width_full()
                .selectable(false)
                .font_size(14.0)
                .padding_horiz(12.0)
                .padding_vert(7.0)
                .color(if enabled {
                    theme.text
                } else {
                    theme.text_muted
                })
                .background(theme.menu_popup_bg)
                .apply_if(enabled, |s| {
                    s.hover(|s| s.background(theme.menu_popup_selected_bg))
                })
        })
        .on_event_stop(listener::Click, move |_, _| {
            if enabled {
                action();
            }
        })
}

fn editor_context_menu_content(state: AppState, document_id: DocumentId) -> impl IntoView {
    let document = state
        .document_by_id_untracked(document_id)
        .expect("context menu document");
    let can_cut_or_copy = editor_has_selection(&document);
    let can_paste = editor_can_paste();
    let cut_state = state.clone();
    let copy_state = state.clone();
    let paste_state = state.clone();

    Stack::vertical((
        context_menu_item(
            "Cut",
            can_cut_or_copy,
            {
                let document = document.clone();
                move || {
                    run_editor_clipboard_command(&document, EditCommand::ClipboardCut);
                    cut_state.close_editor_context_menu();
                }
            },
            state.clone(),
        ),
        context_menu_item(
            "Copy",
            can_cut_or_copy,
            {
                let document = document.clone();
                move || {
                    run_editor_clipboard_command(&document, EditCommand::ClipboardCopy);
                    copy_state.close_editor_context_menu();
                }
            },
            state.clone(),
        ),
        context_menu_item(
            "Paste",
            can_paste,
            {
                let document = document;
                move || {
                    run_editor_clipboard_command(&document, EditCommand::ClipboardPaste);
                    paste_state.close_editor_context_menu();
                }
            },
            state.clone(),
        ),
    ))
    .style(move |s| {
        let theme = state.app_theme();
        s.min_width(180.0)
            .padding_vert(6.0)
            .border(1.0)
            .border_color(theme.border)
            .background(theme.menu_popup_bg)
    })
    .on_event_stop(listener::PointerDown, move |_, _| {})
}

pub(crate) fn editor_context_menu_overlay(state: AppState) -> impl IntoView {
    Container::derived({
        let state = state.clone();
        move || {
            let Some(menu) = state.editor_context_menu.get() else {
                return Empty::new().into_any();
            };

            Container::new(editor_context_menu_content(state.clone(), menu.document_id))
                .style(move |s| {
                    s.fixed()
                        .inset_left(menu.position.x.round())
                        .inset_top(menu.position.y.round())
                        .z_index(400)
                })
                .into_any()
        }
    })
    .style(move |s| {
        s.fixed()
            .size_full()
            .z_index(350)
            .apply_if(state.editor_context_menu.get().is_none(), |s| s.hide())
    })
    .on_event_stop(listener::PointerDown, move |_, _| {
        state.close_editor_context_menu();
    })
}

fn app_editor_content(
    editor: RwSignal<Editor>,
    document: DocumentState,
    state: AppState,
    is_active: impl Fn(bool) -> bool + 'static + Copy,
    handle_key_event: impl Fn(KeypressKey) -> CommandExecuted + 'static,
) -> impl IntoView {
    let ed = editor.get_untracked();
    let cursor = ed.cursor;
    let scroll_delta = ed.scroll_delta;
    let scroll_to = ed.scroll_to;
    let window_origin = ed.window_origin;
    let viewport = ed.viewport;

    Scroll::new({
        let editor_content_view =
            editor_view(editor, is_active).style(move |s| s.absolute().cursor(CursorStyle::Text));
        let content_id = editor_content_view.id();
        ed.editor_view_id.set(Some(content_id));

        editor_content_view
            .on_event_cont(listener::FocusGained, move |_, _| {
                editor.with_untracked(|ed| ed.editor_view_focused.notify())
            })
            .on_event_cont(listener::FocusLost, move |_, _| {
                editor.with_untracked(|ed| ed.editor_view_focus_lost.notify())
            })
            .on_event_cont(listener::PointerDown, {
                let state = state.clone();
                move |cx,
                      PointerButtonEvent {
                          button,
                          state: pointer_state,
                          pointer,
                      }| {
                    content_id.request_focus();
                    content_id.request_paint();

                    match button {
                        Some(PointerButton::Primary) => {
                            state.close_editor_context_menu();
                            if let Some(pointer_id) = pointer.pointer_id {
                                cx.request_pointer_capture(pointer_id);
                            }
                            editor.get_untracked().pointer_down_primary(pointer_state);
                        }
                        Some(PointerButton::Secondary) => {
                            let current_editor = editor.get_untracked();
                            move_caret_for_secondary_click(&current_editor, pointer_state);
                        }
                        _ => {}
                    }
                }
            })
            .on_event_cont(listener::PointerMove, move |_cx, pu| {
                let editor = editor.get_untracked();
                if editor.active.get_untracked() {
                    content_id.request_paint();
                }
                editor.pointer_move(&pu.current);
            })
            .on_event_cont(listener::PointerUp, {
                move |_cx,
                      PointerButtonEvent {
                          button,
                          state: pointer_state,
                          ..
                      }| {
                    editor.get_untracked().pointer_up(pointer_state);

                    match button {
                        Some(PointerButton::Secondary) => {
                            open_editor_context_menu(
                                &state,
                                &document,
                                context_menu_anchor(content_id, pointer_state),
                            );
                        }
                        Some(PointerButton::Primary) => {
                            state.close_editor_context_menu();
                        }
                        _ => {}
                    }
                }
            })
            .on_event(
                listener::KeyDown,
                move |cx, KeyboardEvent { key, modifiers, .. }| {
                    if !cx.window_state.is_focused(content_id) {
                        return EventPropagation::Continue;
                    }
                    if *key == Key::Named(NamedKey::Tab) {
                        cx.prevent_default();
                    }
                    if handle_key_event(KeypressKey {
                        key: key.clone(),
                        modifiers: *modifiers,
                    }) == CommandExecuted::Yes
                    {
                        cx.window_state.request_paint(cx.target);
                    }

                    let mut mods = *modifiers;
                    mods.set(Modifiers::SHIFT, false);
                    mods.set(Modifiers::ALT, false);
                    #[cfg(target_os = "macos")]
                    mods.set(Modifiers::ALT, false);

                    if mods.is_empty()
                        && let Key::Character(c) = &key
                    {
                        cx.window_state.request_paint(cx.target);
                        editor.get_untracked().receive_char(c);
                    }
                    EventPropagation::Stop
                },
            )
            .style(|s| s.min_size_full())
    })
    .on_event_stop(VisualChanged::listener(), move |_cx, change| {
        window_origin.set(change.visual_window_origin());
    })
    .scroll_to(move || scroll_to.get().map(Vec2::to_point))
    .scroll_delta(move || scroll_delta.get())
    .ensure_visible(move || {
        let editor = editor.get_untracked();
        let cursor = cursor.get();
        let offset = cursor.offset();
        let _ = editor.doc_track();

        let LineRegion { x, width, rvline } =
            cursor_caret(&editor, offset, !cursor.is_insert(), cursor.affinity());

        let line_height = f64::from(editor.line_height(0));
        let vline = editor.vline_of_rvline(rvline);
        let rect =
            Rect::from_origin_size((x, vline.get() as f64 * line_height), (width, line_height))
                .inflate(10.0, 1.0);

        let viewport = viewport.get_untracked();
        let smallest_distance = (viewport.y0 - rect.y0)
            .abs()
            .min((viewport.y1 - rect.y0).abs())
            .min((viewport.y0 - rect.y1).abs())
            .min((viewport.y1 - rect.y1).abs());
        let biggest_distance = (viewport.y0 - rect.y0)
            .abs()
            .max((viewport.y1 - rect.y0).abs())
            .max((viewport.y0 - rect.y1).abs())
            .max((viewport.y1 - rect.y1).abs());
        let jump_to_middle =
            biggest_distance > viewport.height() && smallest_distance > viewport.height() / 2.0;

        if jump_to_middle {
            rect.inflate(0.0, viewport.height() / 2.0)
        } else {
            let mut rect = rect;
            let cursor_surrounding_lines = editor.es.with(|s| s.cursor_surrounding_lines()) as f64;
            rect.y0 -= cursor_surrounding_lines * line_height;
            rect.y1 += cursor_surrounding_lines * line_height;
            rect
        }
    })
    .style(|s| s.size_pct(100.0, 100.0))
}

fn app_editor_container_view(
    editor: RwSignal<Editor>,
    document: DocumentState,
    state: AppState,
    is_active: impl Fn(bool) -> bool + 'static + Copy,
    handle_key_event: impl Fn(KeypressKey) -> CommandExecuted + 'static,
) -> impl IntoView {
    Stack::new((
        editor_gutter(editor),
        app_editor_content(editor, document, state, is_active, handle_key_event),
    ))
    .style(|s| {
        s.absolute()
            .size_pct(100.0, 100.0)
            .overflow_x(Overflow::Clip)
            .overflow_y(Overflow::Clip)
    })
    .on_cleanup(move || {
        let editor = editor.get_untracked();
        editor.cx.get().dispose();
    })
}

fn document_editor_view(document: DocumentState, state: AppState) -> impl IntoView {
    let editor_sig = RwSignal::new(document.editor.clone());
    let document_id = document.id();
    let focus_document = document.clone();
    let focus_state = state.clone();

    Effect::new(move |_| {
        let is_active = focus_state.documents.get().active_document_id() == Some(document_id);
        let view_id = focus_document.editor.editor_view_id.get();

        if is_active && let Some(view_id) = view_id {
            view_id.request_focus();
        }
    });

    app_editor_container_view(
        editor_sig,
        document,
        state.clone(),
        |_| true,
        default_key_handler(editor_sig),
    )
    .style({
        move |s| {
            let theme = state.app_theme();
            s.apply(editor_theme_style(theme))
                .width_full()
                .min_size(0, 0)
                .flex_grow(1.0)
                .border(1.0)
                .border_color(theme.border)
        }
    })
}

pub(crate) fn tab_content_view(state: AppState) -> impl IntoView {
    let active_state = state.clone();
    let documents_state = state.clone();

    tab(
        move || active_state.active_index(),
        move || documents_state.documents(),
        DocumentState::id,
        move |document| document_editor_view(document, state.clone()),
    )
    .style(|s| s.width_full().min_size(0, 0).flex_grow(1.0))
}

#[cfg(test)]
mod tests {
    use floem::{
        headless::{HeadlessHarness, TestRoot},
        prelude::{SignalGet, SignalUpdate, SignalWith},
        reactive::Scope,
        views::editor::core::cursor::CursorAffinity,
    };

    use crate::{
        persistence::{config::AppConfig, recent_files::RecentFiles},
        workspace::state::DocumentState,
        workspace::{
            AppState, DocumentId, DocumentSet, activate_document, create_document_state,
        },
    };

    use super::{
        EditCommand, EditorCommand, editor_has_selection, move_caret_for_secondary_click,
        run_editor_clipboard_command, run_editor_command, tab_content_view,
    };
    use floem::views::editor::core::command::MultiSelectionCommand;
    use floem::ui_events::pointer::PointerState;

    #[test]
    fn tab_content_view_builds_editor_views_and_tracks_active_document() {
        let root = TestRoot::new();
        let state = test_state_with_two_documents();
        let document_a = state
            .document_by_id_untracked(DocumentId::initial())
            .expect("document a");
        let document_b = state
            .documents()
            .into_iter()
            .find(|document| document.id() != DocumentId::initial())
            .expect("document b");
        let mut harness =
            HeadlessHarness::new_with_size(root, tab_content_view(state.clone()), 920.0, 680.0);

        harness.rebuild();

        let editor_a_view_id = document_a
            .editor
            .editor_view_id
            .get_untracked()
            .expect("editor a view id");
        let editor_b_view_id = document_b
            .editor
            .editor_view_id
            .get_untracked()
            .expect("editor b view id");
        assert_ne!(editor_a_view_id, editor_b_view_id);
        assert_eq!(state.active_index(), Some(0));

        state.set_active_document(document_b.id());
        harness.process_update_no_paint();

        assert_eq!(state.active_index(), Some(1));
    }

    #[test]
    fn editor_context_menu_disables_cut_and_copy_without_a_selection() {
        let _root = TestRoot::new();
        let state = test_state_with_two_documents();
        let document = state
            .document_by_id_untracked(DocumentId::initial())
            .expect("document");

        assert!(!editor_has_selection(&document));

        document.editor.cursor.update(|cursor| {
            cursor.set_offset(0, CursorAffinity::Backward, false, false);
            cursor.set_offset(3, CursorAffinity::Backward, true, false);
        });

        assert!(editor_has_selection(&document));
    }

    #[test]
    fn cut_command_removes_selected_text() {
        let _root = TestRoot::new();
        let state = test_state_with_two_documents();
        let document = state
            .document_by_id_untracked(DocumentId::initial())
            .expect("document");

        run_editor_command(
            &document,
            EditorCommand::MultiSelection(MultiSelectionCommand::SelectAll),
        );
        assert!(editor_has_selection(&document));

        run_editor_clipboard_command(&document, EditCommand::ClipboardCut);

        assert_eq!(document.editor.doc().text().to_string(), "");
        assert!(document.editor.doc().is_dirty());
    }

    #[test]
    fn secondary_click_outside_selection_moves_caret_to_click() {
        let root = TestRoot::new();
        let state = test_state_with_two_documents();
        let document = state
            .document_by_id_untracked(DocumentId::initial())
            .expect("document");
        let mut harness =
            HeadlessHarness::new_with_size(root, tab_content_view(state.clone()), 920.0, 680.0);

        harness.rebuild();

        document.editor.cursor.update(|cursor| {
            cursor.set_offset(0, CursorAffinity::Backward, false, false);
            cursor.set_offset(5, CursorAffinity::Backward, true, false);
        });

        let pointer_state = pointer_state_for_offset(&document, 8);
        move_caret_for_secondary_click(&document.editor, &pointer_state);

        let doc = document.editor.doc();
        let selection = document
            .editor
            .cursor
            .with_untracked(|cursor| cursor.edit_selection(&doc.rope_text()));
        assert!(selection.is_caret());
        assert_eq!(selection.min_offset(), 8);
    }

    #[test]
    fn secondary_click_inside_selection_keeps_selection() {
        let root = TestRoot::new();
        let state = test_state_with_two_documents();
        let document = state
            .document_by_id_untracked(DocumentId::initial())
            .expect("document");
        let mut harness =
            HeadlessHarness::new_with_size(root, tab_content_view(state.clone()), 920.0, 680.0);

        harness.rebuild();

        document.editor.cursor.update(|cursor| {
            cursor.set_offset(0, CursorAffinity::Backward, false, false);
            cursor.set_offset(5, CursorAffinity::Backward, true, false);
        });

        let pointer_state = pointer_state_for_offset(&document, 3);
        move_caret_for_secondary_click(&document.editor, &pointer_state);

        let doc = document.editor.doc();
        let selection = document
            .editor
            .cursor
            .with_untracked(|cursor| cursor.edit_selection(&doc.rope_text()));
        assert!(!selection.is_caret());
        assert_eq!(selection.min_offset(), 0);
        assert_eq!(selection.first().expect("selection").max(), 5);
    }

    fn pointer_state_for_offset(document: &DocumentState, offset: usize) -> PointerState {
        let point = document
            .editor
            .line_point_of_offset(offset, CursorAffinity::Backward);
        PointerState {
            position: (point.x + 1.0, point.y).into(),
            scale_factor: 1.0,
            ..Default::default()
        }
    }

    fn test_state_with_two_documents() -> AppState {
        let scope = Scope::new();
        let state = AppState::new(scope, RecentFiles::default(), AppConfig::default());
        let initial_document = create_document_state(
            scope,
            DocumentId::initial(),
            None,
            String::from("first line"),
            state.editor_font_untracked(),
            state.editor_font_size_untracked(),
        );
        state.documents.set(DocumentSet::new(initial_document));

        let second_document = create_document_state(
            state.document_scope,
            state.allocate_document_id(),
            None,
            String::from("second"),
            state.editor_font_untracked(),
            state.editor_font_size_untracked(),
        );
        state.push_document(second_document);
        activate_document(&state, DocumentId::initial());
        state
    }
}
