use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;
use floem::prelude::{SignalGet, SignalUpdate};

use crate::{
    paths::app_data_file_path,
    state::{AppState, save_target_path},
};

const RECENT_FILES_NAME: &str = "recent-files.txt";
const MAX_RECENT_FILES: usize = 10;

#[derive(Clone, Default)]
pub(crate) struct RecentFiles {
    entries: Vec<PathBuf>,
}

impl RecentFiles {
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        self.entries.clone()
    }

    #[cfg(test)]
    pub(crate) fn from_paths<I>(paths: I) -> Self
    where
        I: IntoIterator<Item = PathBuf>,
        I::IntoIter: DoubleEndedIterator,
    {
        let mut recent_files = Self::default();
        for path in paths.into_iter().rev() {
            recent_files.add_path(&path);
        }
        recent_files
    }

    fn add_path(&mut self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        let path = save_target_path(path);
        self.entries.retain(|existing| !same_path(existing, &path));
        self.entries.insert(0, path);
        self.entries.truncate(MAX_RECENT_FILES);
        true
    }

    fn remove_path(&mut self, path: &Path) -> bool {
        let original_len = self.entries.len();
        self.entries.retain(|existing| !same_path(existing, path));
        self.entries.len() != original_len
    }

    fn clear(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }

        self.entries.clear();
        true
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    save_target_path(left) == save_target_path(right)
}

fn recent_files_path() -> Result<PathBuf, String> {
    app_data_file_path(RECENT_FILES_NAME)
}

fn store_recent_files(recent_files: &RecentFiles) -> Result<(), String> {
    // Tests operate on the in-memory signal only; skip persisting to avoid
    // overwriting the user's real recent-files.txt with temp paths.
    if cfg!(test) {
        return Ok(());
    }

    let path = recent_files_path()?;
    let Some(parent) = path.parent() else {
        return Err(format!(
            "Recent files path has no parent: {}",
            path.display()
        ));
    };

    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;

    let file = AtomicWriteFile::open(&path)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;
    let mut writer = std::io::BufWriter::new(file);

    for recent_path in &recent_files.entries {
        writeln!(writer, "{}", recent_path.display())
            .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;
    }

    writer
        .flush()
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;
    let file = writer
        .into_inner()
        .map_err(|err| format!("Failed to write {}: {}", path.display(), err.into_error()))?;
    file.commit()
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))
}

pub(crate) fn load_recent_files() -> Result<RecentFiles, String> {
    let path = recent_files_path()?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(RecentFiles::default()),
        Err(err) => {
            return Err(format!("Failed to read {}: {err}", path.display()));
        }
    };

    let mut recent_files = RecentFiles::default();
    for line in contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .rev()
    {
        recent_files.add_path(Path::new(line));
    }

    Ok(recent_files)
}

pub(crate) fn record_recent_file(state: &AppState, path: &Path) {
    let mut recent_files = state.recent_files.get_untracked();
    if !recent_files.add_path(path) {
        return;
    }

    if let Err(err) = store_recent_files(&recent_files) {
        state.status_message.set(Some(err));
        return;
    }

    state.recent_files.set(recent_files);
}

pub(crate) fn remove_recent_file(state: &AppState, path: &Path) {
    let mut recent_files = state.recent_files.get_untracked();
    if !recent_files.remove_path(path) {
        return;
    }

    if let Err(err) = store_recent_files(&recent_files) {
        state.status_message.set(Some(err));
        return;
    }

    state.recent_files.set(recent_files);
}

pub(crate) fn clear_recent_files(state: &AppState) {
    let mut recent_files = state.recent_files.get_untracked();
    if !recent_files.clear() {
        return;
    }

    if let Err(err) = store_recent_files(&recent_files) {
        state.status_message.set(Some(err));
        return;
    }

    state.recent_files.set(recent_files);
    state
        .status_message
        .set(Some("Cleared recent files".to_string()));
}
