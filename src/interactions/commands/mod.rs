use std::{collections::HashMap, sync::Arc};

use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    http::interaction::InteractionResponse,
};

use crate::AppState;

pub trait Command {
    fn options() -> Vec<CommandOption>;

    fn from_interaction_data(data: &InteractionData) -> Self;
}

#[derive(Debug)]
pub struct CommandOption {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}

type CommandHandler<C> = fn(C, Arc<Interaction>, Arc<AppState>) -> InteractionResponse;

pub struct CommandExecutor {
    commands: HashMap<
        String,
        Box<dyn Fn(Arc<Interaction>, Arc<AppState>) -> InteractionResponse + Send + Sync>,
    >,
}

impl CommandExecutor {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register<C: Command + 'static>(&mut self, name: &str, executor: CommandHandler<C>) {
        self.commands.insert(
            name.to_string(),
            Box::new(move |interaction: Arc<Interaction>, state: Arc<AppState>| {
                let command_data = C::from_interaction_data(
                    interaction
                        .data
                        .as_ref()
                        .expect("Interaction data should be present"),
                );
                executor(command_data, interaction, state)
            }),
        );
    }
}
