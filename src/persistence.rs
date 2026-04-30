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
const CUSTOM_THEME_FILE_NAME: &str = "theme.json";
const RECENT_FILES_NAME: &str = "recent-files.txt";
const MAX_RECENT_FILES: usize = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GpuiThemePreference {
    DefaultLight,
    DefaultDark,
    Custom,
}

impl Default for GpuiThemePreference {
    fn default() -> Self {
        Self::DefaultLight
    }
}

impl GpuiThemePreference {
    pub(crate) fn config_value(self) -> &'static str {
        match self {
            Self::DefaultLight => "default_light",
            Self::DefaultDark => "default_dark",
            Self::Custom => "custom",
        }
    }

    pub(crate) fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "default_light" | "light" => Some(Self::DefaultLight),
            "default_dark" | "dark" => Some(Self::DefaultDark),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorMode {
    Basic,
    CodeEditor,
}

impl Default for EditorMode {
    fn default() -> Self {
        Self::CodeEditor
    }
}

impl EditorMode {
    pub(crate) fn config_value(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::CodeEditor => "code_editor",
        }
    }

    pub(crate) fn from_config_value(value: &str) -> Option<Self> {
        match value {
            "basic" | "plain" | "plain_text" => Some(Self::Basic),
            "code_editor" | "markdown_code_editor" => Some(Self::CodeEditor),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppConfig {
    pub(crate) gpui_theme: GpuiThemePreference,
    pub(crate) editor_mode: EditorMode,
    pub(crate) editor_font: String,
    pub(crate) editor_font_size: usize,
    main_theme_preference: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gpui_theme: GpuiThemePreference::DefaultLight,
            editor_mode: EditorMode::default(),
            editor_font: default_editor_font(),
            editor_font_size: default_editor_font_size(),
            main_theme_preference: None,
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
                "theme" => {
                    if !value.is_empty() {
                        config.main_theme_preference = Some(value.to_string());
                    }
                }
                "gpui_theme" => {
                    let Some(theme) = GpuiThemePreference::from_config_value(value) else {
                        return Err(format!("Invalid gpui_theme value: {value}"));
                    };
                    config.gpui_theme = theme;
                }
                "editor_mode" => {
                    let Some(editor_mode) = EditorMode::from_config_value(value) else {
                        return Err(format!("Invalid editor_mode value: {value}"));
                    };
                    config.editor_mode = editor_mode;
                }
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
        let mut text = String::from("# lazy-markdown user configuration\n");
        if let Some(theme) = &self.main_theme_preference {
            text.push_str(&format!("theme = \"{theme}\"\n"));
        }
        text.push_str(&format!(
            "gpui_theme = \"{}\"\neditor_mode = \"{}\"\neditor_font = \"{}\"\neditor_font_size = {}\n",
            self.gpui_theme.config_value(),
            self.editor_mode.config_value(),
            normalize_editor_font(&self.editor_font),
            normalize_editor_font_size(self.editor_font_size)
        ));
        text
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

pub(crate) fn custom_theme_path() -> Result<PathBuf, String> {
    Ok(app_config_directory()?.join(CUSTOM_THEME_FILE_NAME))
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
    use super::{AppConfig, EditorMode, GpuiThemePreference};
    use crate::preferences::{MAX_EDITOR_FONT_SIZE, MONOSPACE_FONT, SYSTEM_DEFAULT_FONT};

    #[test]
    fn config_round_trip_keeps_editor_font_preferences() {
        let config = AppConfig {
            gpui_theme: GpuiThemePreference::Custom,
            editor_mode: EditorMode::Basic,
            editor_font: MONOSPACE_FONT.to_string(),
            editor_font_size: 19,
            main_theme_preference: Some("dark".to_string()),
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

    #[test]
    fn config_accepts_gpui_theme_preferences() {
        let config = AppConfig::from_str("gpui_theme = \"custom\"\n").unwrap();

        assert_eq!(config.gpui_theme, GpuiThemePreference::Custom);
    }

    #[test]
    fn config_accepts_editor_mode_preferences() {
        let config = AppConfig::from_str("editor_mode = \"basic\"\n").unwrap();
        assert_eq!(config.editor_mode, EditorMode::Basic);

        let config = AppConfig::from_str("editor_mode = \"code_editor\"\n").unwrap();
        assert_eq!(config.editor_mode, EditorMode::CodeEditor);
    }
}
