use gpui::{Context, Entity, SharedString, Subscription, Window};
use gpui_component::{
    WindowExt, button::ButtonVariant, dialog::DialogButtonProps, menu::AppMenuBar,
};

use crate::{
    New, Open, Save, SaveAs,
    documents::{DocumentId, ProbeDocument, SAMPLE_MARKDOWN},
    menus::install_app_menus,
};

pub(crate) struct ProbeWindow {
    pub(crate) documents: Vec<ProbeDocument>,
    pub(crate) active_document_id: Option<DocumentId>,
    pub(crate) next_document_id: DocumentId,
    pub(crate) app_menu_bar: Entity<AppMenuBar>,
    pub(crate) status: SharedString,
    pub(crate) _subscriptions: Vec<Subscription>,
}

impl ProbeWindow {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
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

    pub(crate) fn on_new(&mut self, _: &New, window: &mut Window, cx: &mut Context<Self>) {
        self.create_new_document(window, cx);
    }

    pub(crate) fn on_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt_open_document(window, cx);
    }

    pub(crate) fn on_save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        let Some(document) = self.active_document() else {
            return;
        };
        let document_id = document.id;

        match document.current_path.clone() {
            Some(path) => self.save_to_path(document_id, &path, cx),
            None => self.prompt_save_as(document_id, window, cx),
        }
    }

    pub(crate) fn on_save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(document_id) = self.active_document_id {
            self.prompt_save_as(document_id, window, cx);
        }
    }

    pub(crate) fn should_close_window(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
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
}
