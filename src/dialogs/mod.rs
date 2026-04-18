mod confirm;

use floem::window::WindowId;

use crate::workspace::DocumentId;

pub(crate) use confirm::dialog_overlay;

#[derive(Clone)]
pub(crate) enum ActiveDialog {
    ConfirmCloseDocument {
        document_id: DocumentId,
    },
    ConfirmCloseWindow {
        window_id: WindowId,
        remaining_documents: Vec<DocumentId>,
    },
    Message {
        title: String,
        message: String,
    },
}

impl ActiveDialog {
    pub(crate) fn title_text(&self) -> String {
        match self {
            ActiveDialog::ConfirmCloseDocument { .. } | ActiveDialog::ConfirmCloseWindow { .. } => {
                "Unsaved changes".to_string()
            }
            ActiveDialog::Message { title, .. } => title.clone(),
        }
    }

    pub(crate) fn message_text(&self) -> String {
        match self {
            ActiveDialog::ConfirmCloseDocument { .. } => {
                "Save your changes before closing this tab?".to_string()
            }
            ActiveDialog::ConfirmCloseWindow { .. } => {
                "Save your changes before closing this window?".to_string()
            }
            ActiveDialog::Message { message, .. } => message.clone(),
        }
    }

    pub(crate) fn shows_document_path(&self) -> bool {
        matches!(
            self,
            ActiveDialog::ConfirmCloseDocument { .. } | ActiveDialog::ConfirmCloseWindow { .. }
        )
    }

    pub(crate) fn needs_save_decision(&self) -> bool {
        matches!(
            self,
            ActiveDialog::ConfirmCloseDocument { .. } | ActiveDialog::ConfirmCloseWindow { .. }
        )
    }
}
