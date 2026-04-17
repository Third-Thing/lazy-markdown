use crate::{
    commands::{CommandRegistry, ModuleRegistry, register_builtin_commands},
    persistence::config::{AppConfig, load_app_config},
};

#[derive(Clone)]
pub(crate) struct AppBootstrap {
    pub(crate) command_registry: CommandRegistry,
    pub(crate) app_config: AppConfig,
    pub(crate) app_config_error: Option<String>,
}

impl AppBootstrap {
    pub(crate) fn load() -> Result<Self, String> {
        let mut registry = ModuleRegistry::new();
        register_builtin_commands(registry.commands_mut())?;
        let (app_config, app_config_error) = match load_app_config() {
            Ok(app_config) => (app_config, None),
            Err(err) => (AppConfig::default(), Some(err)),
        };

        Ok(Self {
            command_registry: registry.commands().clone(),
            app_config,
            app_config_error,
        })
    }
}
