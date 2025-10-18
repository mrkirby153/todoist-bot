use std::{collections::HashMap, pin::Pin, sync::Arc};

use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::Interaction,
    },
    http::interaction::InteractionResponse,
};
use twilight_util::builder::command::CommandBuilder;

pub mod commands;
pub mod verifier;

type AsyncHandler<T> = Box<
    dyn Fn(Arc<Interaction>, Arc<T>) -> Pin<Box<dyn Future<Output = InteractionResponse> + Send>>
        + Send
        + Sync,
>;

/// Commands that can be used via a context menu.
#[derive(Default)]
pub struct ContextCommands<T> {
    commands: HashMap<String, AsyncHandler<T>>,
}

impl<T> ContextCommands<T> {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register<F, Fut>(&mut self, command: &str, handler: F)
    where
        F: Fn(Arc<Interaction>, Arc<T>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = InteractionResponse> + Send + 'static,
    {
        let handler = Box::new(move |interaction, state| {
            Box::pin(handler(interaction, state))
                as Pin<Box<dyn Future<Output = InteractionResponse> + Send>>
        });
        self.commands.insert(command.to_string(), handler);
    }

    pub fn get(&self, name: &str) -> Option<&AsyncHandler<T>> {
        self.commands.get(name)
    }
}

impl<T> From<&ContextCommands<T>> for Vec<Command> {
    fn from(context_commands: &ContextCommands<T>) -> Vec<Command> {
        context_commands
            .commands
            .keys()
            .map(|name| CommandBuilder::new(name, "", CommandType::Message).build())
            .collect()
    }
}
