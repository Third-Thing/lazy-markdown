use std::{
    fs,
    io::{ErrorKind, Write},
};

use atomic_write_file::AtomicWriteFile;

use crate::{
    persistence::paths::app_config_file_path,
    preferences::{editor_font::{default_editor_font, default_editor_font_size, normalize_editor_font_size}, theme::ThemePreference},
};

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AppConfig {
    pub(crate) theme_preference: ThemePreference,
    pub(crate) editor_font: String,
    pub(crate) editor_font_size: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_preference: ThemePreference::FollowOs,
            editor_font: default_editor_font(),
            editor_font_size: default_editor_font_size(),
        }
    }
}

impl AppConfig {
    fn parse(contents: &str) -> Result<Self, String> {
        let mut config = Self::default();

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                return Err(format!("Invalid config line: {line}"));
            };

            let key = key.trim();
            let value = value.trim().trim_matches('"');

            match key {
                "theme" => {
                    let Some(theme_preference) = ThemePreference::from_config_value(value) else {
                        return Err(format!("Invalid theme value: {value}"));
                    };
                    config.theme_preference = theme_preference;
                }
                "editor_font" => {
                    if value.is_empty() {
                        return Err("Invalid editor_font value: empty string".to_string());
                    };
                    config.editor_font = value.to_string();
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

    fn encode(&self) -> String {
        format!(
            "# lazy-markdown user configuration\ntheme = \"{}\"\neditor_font = \"{}\"\neditor_font_size = {}\n",
            self.theme_preference.config_value(),
            self.editor_font,
            self.editor_font_size
        )
    }
}

pub(crate) fn load_app_config() -> Result<AppConfig, String> {
    let path = app_config_file_path(CONFIG_FILE_NAME)?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(AppConfig::default()),
        Err(err) => {
            return Err(format!("Failed to read {}: {err}", path.display()));
        }
    };

    AppConfig::parse(&contents)
}

pub(crate) fn store_app_config(config: AppConfig) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }

    let path = app_config_file_path(CONFIG_FILE_NAME)?;
    let Some(parent) = path.parent() else {
        return Err(format!("Config path has no parent: {}", path.display()));
    };

    fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;

    let file = AtomicWriteFile::open(&path)
        .map_err(|err| format!("Failed to write {}: {err}", path.display()))?;
    let mut writer = std::io::BufWriter::new(file);

    writer
        .write_all(config.encode().as_bytes())
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

#[cfg(test)]
mod tests {
    use crate::preferences::theme::ThemePreference;

    use super::AppConfig;

    #[test]
    fn config_round_trip_keeps_editor_font() {
        let config = AppConfig {
            theme_preference: ThemePreference::Dark,
            editor_font: "monospace".to_string(),
            editor_font_size: 19,
        };

        let encoded = config.encode();
        let decoded = AppConfig::parse(&encoded).expect("parse config");

        assert_eq!(decoded, config);
    }
}
