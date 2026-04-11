use crate::commands::{CommandRegistry, ModuleRegistry, register_builtin_commands};

#[derive(Clone)]
pub(crate) struct AppBootstrap {
    pub(crate) command_registry: CommandRegistry,
}

impl AppBootstrap {
    pub(crate) fn load() -> Result<Self, String> {
        let mut registry = ModuleRegistry::new();
        register_builtin_commands(registry.commands_mut())?;

        Ok(Self {
            command_registry: registry.commands().clone(),
        })
    }
}
