use std::{env, path::PathBuf};

const APP_DIR_NAME: &str = "lazy-markdown";

pub(crate) fn app_data_directory() -> Result<PathBuf, String> {
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

pub(crate) fn app_config_directory() -> Result<PathBuf, String> {
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

pub(crate) fn app_data_file_path(file_name: &str) -> Result<PathBuf, String> {
    Ok(app_data_directory()?.join(file_name))
}

pub(crate) fn app_config_file_path(file_name: &str) -> Result<PathBuf, String> {
    Ok(app_config_directory()?.join(file_name))
}
