use std::{
    fs,
    io::{ErrorKind, Write},
};

use atomic_write_file::AtomicWriteFile;

use crate::{paths::app_config_file_path, theme::ThemePreference};

const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AppConfig {
    pub(crate) theme_preference: ThemePreference,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_preference: ThemePreference::FollowOs,
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
                _ => {}
            }
        }

        Ok(config)
    }

    fn encode(self) -> String {
        format!(
            "# lazy-markdown user configuration\ntheme = \"{}\"\n",
            self.theme_preference.config_value()
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
