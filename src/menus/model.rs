use std::rc::Rc;

use crate::workspace::TopLevelMenuId;

#[derive(Clone)]
pub(crate) struct AppMenuModel {
    pub(crate) id: TopLevelMenuId,
    pub(crate) title: String,
    pub(crate) entries: Vec<AppMenuEntry>,
}

impl AppMenuModel {
    pub(crate) fn new(
        id: TopLevelMenuId,
        title: impl Into<String>,
        entries: Vec<AppMenuEntry>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            entries,
        }
    }

    pub(crate) fn first_selectable_index(&self) -> Option<usize> {
        self.entries.iter().position(AppMenuEntry::is_selectable)
    }

    pub(crate) fn next_selectable_index(&self, current_index: usize, step: isize) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }

        let len = self.entries.len() as isize;
        let mut index = current_index as isize;

        for _ in 0..self.entries.len() {
            index = (index + step).rem_euclid(len);
            if self.entries[index as usize].is_selectable() {
                return Some(index as usize);
            }
        }

        None
    }
}

#[derive(Clone)]
pub(crate) enum AppMenuEntry {
    Separator,
    Item(AppMenuItem),
}

impl AppMenuEntry {
    pub(crate) fn item(
        title: impl Into<String>,
        action: impl Fn(&crate::workspace::AppState) + 'static,
    ) -> Self {
        Self::Item(AppMenuItem::new(title, action))
    }

    pub(crate) fn disabled(title: impl Into<String>) -> Self {
        Self::Item(AppMenuItem::disabled(title))
    }

    pub(crate) fn is_selectable(&self) -> bool {
        matches!(self, Self::Item(item) if item.enabled && item.action.is_some())
    }
}

#[derive(Clone)]
pub(crate) struct AppMenuItem {
    pub(crate) title: String,
    pub(crate) enabled: bool,
    pub(crate) action: Option<Rc<dyn Fn(&crate::workspace::AppState)>>,
}

impl AppMenuItem {
    fn new(
        title: impl Into<String>,
        action: impl Fn(&crate::workspace::AppState) + 'static,
    ) -> Self {
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

#[derive(Clone)]
pub(crate) struct PopupRow {
    pub(crate) index: usize,
    pub(crate) entry: AppMenuEntry,
}
