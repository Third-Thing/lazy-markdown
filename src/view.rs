use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
    Styled as _, Window, div, px, relative, FontWeight,
};
use gpui_component::{
    ActiveTheme as _, IconName, Root, Sizable as _,
    button::{Button, ButtonVariants as _},
    input::Input,
    tab::{Tab, TabBar},
    v_flex,
};

use crate::{preferences::editor_font_family, window::AppWindow};

impl Render for AppWindow {
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
        let editor_font_family = editor_font_family(
            &self.app_config.editor_font,
            &self.available_font_families,
            cx,
        );
        let editor_font_size = px(self.app_config.editor_font_size as f32);

        v_flex()
            .id("lazy-markdown-gpui")
            .size_full()
            .relative()
            .bg(cx.theme().background)
            .on_action(cx.listener(Self::on_new))
            .on_action(cx.listener(Self::on_open))
            .on_action(cx.listener(Self::on_save))
            .on_action(cx.listener(Self::on_save_as))
            .on_action(cx.listener(Self::on_open_recent))
            .on_action(cx.listener(Self::on_clear_recent_files))
            .on_action(cx.listener(Self::on_select_editor_font))
            .on_action(cx.listener(Self::on_select_theme))
            .on_action(cx.listener(Self::on_zoom_in))
            .on_action(cx.listener(Self::on_zoom_out))
            .on_action(cx.listener(Self::on_reset_font_size))
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
                        Some(editor) => Input::new(&editor)
                            .size_full()
                            .font_family(editor_font_family)
                            .text_size(editor_font_size)
                            .font_weight(FontWeight::NORMAL)
                            .line_height(relative(1.45))
                            .into_any_element(),
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
