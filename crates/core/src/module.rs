use crate::{CommandMetadata, CommandRegistry};

#[derive(Clone, Default)]
pub struct ModuleRegistry {
    commands: CommandRegistry,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_command(&mut self, command: CommandMetadata) -> Result<(), String> {
        self.commands.register(command)
    }

    pub fn commands(&self) -> &CommandRegistry {
        &self.commands
    }

    pub fn commands_mut(&mut self) -> &mut CommandRegistry {
        &mut self.commands
    }
}
