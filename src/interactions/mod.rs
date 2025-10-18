use std::{collections::HashMap, pin::Pin};

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

type AsyncFn = Box<
    dyn Fn(Interaction) -> Pin<Box<dyn Future<Output = InteractionResponse> + Send>> + Send + Sync,
>;

/// Commands that can be used via a context menu.
#[derive(Default)]
pub struct ContextCommands {
    commands: HashMap<ContextCommandBuilder, AsyncFn>,
}

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct ContextCommandBuilder {
    name: String,
    description: Option<String>,
}

impl ContextCommands {
    pub fn register<F, Fut>(&mut self, command: ContextCommandBuilder, handler: F)
    where
        F: Fn(Interaction) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = InteractionResponse> + Send + 'static,
    {
        let handler = Box::new(move |interaction| {
            Box::pin(handler(interaction))
                as Pin<Box<dyn Future<Output = InteractionResponse> + Send>>
        });
        self.commands.insert(command, handler);
    }

    pub fn get(&self, name: &str) -> Option<&AsyncFn> {
        self.commands
            .iter()
            .find(|item| item.0.name == name)
            .map(|item| item.1)
    }

    pub async fn execute(
        &self,
        name: &str,
        interaction: Interaction,
    ) -> Option<InteractionResponse> {
        if let Some(f) = self.get(name) {
            Some((f)(interaction).await)
        } else {
            None
        }
    }
}

impl From<ContextCommands> for Vec<Command> {
    fn from(context_commands: ContextCommands) -> Vec<Command> {
        context_commands
            .commands
            .keys()
            .map(|builder| {
                let description = if let Some(desc) = &builder.description {
                    desc.as_str()
                } else {
                    ""
                };
                CommandBuilder::new(&builder.name, description, CommandType::Message).build()
            })
            .collect()
    }
}

impl ContextCommandBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: None,
        }
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
}
