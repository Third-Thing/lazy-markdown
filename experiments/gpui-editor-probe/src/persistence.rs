use std::{
    env, fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;

use crate::preferences::{
    default_editor_font, default_editor_font_size, normalize_editor_font,
    normalize_editor_font_size,
};

const APP_DIR_NAME: &str = "lazy-markdown";
const CONFIG_FILE_NAME: &str = "config.toml";
const RECENT_FILES_NAME: &str = "recent-files.txt";
const MAX_RECENT_FILES: usize = 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppConfig {
    pub(crate) editor_font: String,
    pub(crate) editor_font_size: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            editor_font: default_editor_font(),
            editor_font_size: default_editor_font_size(),
        }
    }
}

impl AppConfig {
    fn from_str(contents: &str) -> Result<Self, String> {
        let mut config = Self::default();

        for line in contents.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("Invalid config line `{line}`"));
            };
            let key = key.trim();
            let value = value.trim().trim_matches('"');

            match key {
                "editor_font" => {
                    if value.is_empty() {
                        return Err("Invalid editor_font value: empty string".to_string());
                    }
                    config.editor_font = normalize_editor_font(value);
                }
                "editor_font_size" => {
                    let requested_size = value.parse::<usize>().map_err(|err| {
                        format!("Invalid editor_font_size value `{value}`: {err}")
                    })?;
                    config.editor_font_size = normalize_editor_font_size(requested_size);
                }
                _ => {}
            }
        }

        Ok(config)
    }

    fn to_text(&self) -> String {
        format!(
            "# lazy-markdown user configuration\neditor_font = \"{}\"\neditor_font_size = {}\n",
            normalize_editor_font(&self.editor_font),
            normalize_editor_font_size(self.editor_font_size)
        )
    }
}

#[derive(Clone, Default)]
pub(crate) struct RecentFiles {
    entries: Vec<PathBuf>,
}

impl RecentFiles {
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        self.entries.clone()
    }

    pub(crate) fn add_path(&mut self, path: &Path) -> bool {
        if !path.exists() {
            return false;
        }

        let path = save_target_path(path);
        self.entries.retain(|existing| !same_path(existing, &path));
        self.entries.insert(0, path);
        self.entries.truncate(MAX_RECENT_FILES);
        true
    }

    pub(crate) fn remove_path(&mut self, path: &Path) -> bool {
        let original_len = self.entries.len();
        self.entries.retain(|existing| !same_path(existing, path));
        self.entries.len() != original_len
    }

    pub(crate) fn clear(&mut self) -> bool {
        if self.entries.is_empty() {
            return false;
        }

        self.entries.clear();
        true
    }
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

pub(crate) fn load_app_config() -> Result<AppConfig, String> {
    let path = config_file_path()?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(AppConfig::default()),
        Err(err) => return Err(format!("Failed to read {}: {err}", path.display())),
    };

    AppConfig::from_str(&contents)
        .map_err(|err| format!("Failed to parse {}: {err}", path.display()))
}

pub(crate) fn store_app_config(config: &AppConfig) -> Result<(), String> {
    let path = config_file_path()?;
    let Some(parent) = path.parent() else {
        return Err(format!("Config path has no parent: {}", path.display()));
    };

    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;

    write_file_atomic(&path, config.to_text().as_bytes())
}

pub(crate) fn store_recent_files(recent_files: &RecentFiles) -> Result<(), String> {
    let path = recent_files_path()?;
    let Some(parent) = path.parent() else {
        return Err(format!(
            "Recent files path has no parent: {}",
            path.display()
        ));
    };

    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;

    let mut contents = Vec::new();
    for recent_path in &recent_files.entries {
        writeln!(contents, "{}", recent_path.display())
            .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;
    }

    write_file_atomic(&path, &contents)
}

pub(crate) fn write_file_atomic(path: &Path, contents: &[u8]) -> Result<(), String> {
    let file = AtomicWriteFile::open(path)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;
    let mut writer = std::io::BufWriter::new(file);

    writer
        .write_all(contents)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;
    writer
        .flush()
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;

    let file = writer
        .into_inner()
        .map_err(|err| format!("Failed to write {}: {}", path.display(), err.into_error()))?;
    file.commit()
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))
}

fn recent_files_path() -> Result<PathBuf, String> {
    Ok(app_data_directory()?.join(RECENT_FILES_NAME))
}

fn config_file_path() -> Result<PathBuf, String> {
    Ok(app_config_directory()?.join(CONFIG_FILE_NAME))
}

fn app_config_directory() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(base) = env::var_os("APPDATA").or_else(|| env::var_os("LOCALAPPDATA")) {
            return Ok(PathBuf::from(base).join(APP_DIR_NAME));
        }

        return Err("Failed to resolve APPDATA for app config".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let Some(home) = env::var_os("HOME") else {
            return Err("Failed to resolve HOME for app config".to_string());
        };

        return Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(APP_DIR_NAME));
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        if let Some(base) = env::var_os("XDG_CONFIG_HOME") {
            return Ok(PathBuf::from(base).join(APP_DIR_NAME));
        }

        let Some(home) = env::var_os("HOME") else {
            return Err("Failed to resolve HOME for app config".to_string());
        };

        Ok(PathBuf::from(home).join(".config").join(APP_DIR_NAME))
    }
}

fn app_data_directory() -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(base) = env::var_os("LOCALAPPDATA").or_else(|| env::var_os("APPDATA")) {
            return Ok(PathBuf::from(base).join(APP_DIR_NAME));
        }

        return Err("Failed to resolve LOCALAPPDATA for app data".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let Some(home) = env::var_os("HOME") else {
            return Err("Failed to resolve HOME for app data".to_string());
        };

        return Ok(PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join(APP_DIR_NAME));
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        if let Some(base) = env::var_os("XDG_DATA_HOME") {
            return Ok(PathBuf::from(base).join(APP_DIR_NAME));
        }

        let Some(home) = env::var_os("HOME") else {
            return Err("Failed to resolve HOME for app data".to_string());
        };

        Ok(PathBuf::from(home)
            .join(".local")
            .join("share")
            .join(APP_DIR_NAME))
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    save_target_path(left) == save_target_path(right)
}

fn save_target_path(path: &Path) -> PathBuf {
    if path.exists() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;
    use crate::preferences::{MAX_EDITOR_FONT_SIZE, MONOSPACE_FONT, SYSTEM_DEFAULT_FONT};

    #[test]
    fn config_round_trip_keeps_editor_font_preferences() {
        let config = AppConfig {
            editor_font: MONOSPACE_FONT.to_string(),
            editor_font_size: 19,
        };

        assert_eq!(AppConfig::from_str(&config.to_text()).unwrap(), config);
    }

    #[test]
    fn config_normalizes_unknown_font_preferences() {
        let config =
            AppConfig::from_str("editor_font = \"unknown\"\neditor_font_size = 999\n").unwrap();

        assert_eq!(config.editor_font, SYSTEM_DEFAULT_FONT);
        assert_eq!(config.editor_font_size, MAX_EDITOR_FONT_SIZE);
    }
}
