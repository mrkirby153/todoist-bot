use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use crate::interactions::commands::arguments::CommandOption;
use twilight_model::application::command::Command as TwilightCommand;
use twilight_model::channel::message::MessageFlags;
use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    http::interaction::{InteractionResponse, InteractionResponseData},
};
use twilight_util::builder::command::CommandBuilder;
pub mod arguments;

pub trait Command: Send + Sync + 'static + Sized {
    fn options() -> Vec<CommandOption>;
    fn from_interaction_data(data: &InteractionData) -> Result<Self, arguments::Error>;
}

// Trait for type-erased async handlers
trait AsyncHandler<S>: Send + Sync {
    fn handle(
        &self,
        interaction: Arc<Interaction>,
        state: Arc<S>,
    ) -> Pin<Box<dyn Future<Output = InteractionResponse> + Send>>;
}

// Concrete implementation that preserves command type
struct TypedAsyncHandler<C, S, F, Fut>
where
    C: Command,
    F: Fn(C, Arc<Interaction>, Arc<S>) -> Fut + Send + Sync,
    Fut: Future<Output = InteractionResponse> + Send + 'static,
    S: Send + Sync + 'static,
{
    handler: F,
    _phantom: std::marker::PhantomData<(C, S)>,
}

impl<C: Command, S, F, Fut> AsyncHandler<S> for TypedAsyncHandler<C, S, F, Fut>
where
    C: Command,
    S: Send + Sync + 'static,
    F: Fn(C, Arc<Interaction>, Arc<S>) -> Fut + Send + Sync,
    Fut: Future<Output = InteractionResponse> + Send + 'static,
{
    fn handle(
        &self,
        interaction: Arc<Interaction>,
        state: Arc<S>,
    ) -> Pin<Box<dyn Future<Output = InteractionResponse> + Send>> {
        let command_data = C::from_interaction_data(
            interaction
                .data
                .as_ref()
                .expect("Interaction data should be present"),
        );

        let command_data = match command_data {
            Ok(data) => data,
            Err(_) => {
                return Box::pin(async {
                    InteractionResponse {
                        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData {
                            content: Some("Failed to parse command data.".to_string()),
                            flags: Some(MessageFlags::EPHEMERAL),
                            ..Default::default()
                        }),
                    }
                });
            }
        };

        let fut = (self.handler)(command_data, Arc::clone(&interaction), state);
        Box::pin(fut)
    }
}

struct CommandInfo<S>(Box<dyn AsyncHandler<S>>, Vec<CommandOption>)
where
    S: Send + Sync + 'static;

pub struct CommandExecutor<S>
where
    S: Send + Sync + 'static,
{
    commands: HashMap<String, CommandInfo<S>>,
}

impl<S> CommandExecutor<S>
where
    S: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register an async command handler
    pub fn register<C, F, Fut>(&mut self, name: &str, handler: F)
    where
        C: Command,
        F: Fn(C, Arc<Interaction>, Arc<S>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = InteractionResponse> + Send + 'static,
    {
        let handler = TypedAsyncHandler {
            handler,
            _phantom: std::marker::PhantomData,
        };

        self.commands.insert(
            name.to_string(),
            CommandInfo(Box::new(handler), C::options()),
        );
    }

    pub async fn execute(
        &self,
        name: &str,
        interaction: Arc<Interaction>,
        state: Arc<S>,
    ) -> Option<InteractionResponse> {
        let handler = self.commands.get(name)?;
        Some(handler.0.handle(interaction, state).await)
    }

    pub fn build_commands(&self) -> Vec<TwilightCommand> {
        self.commands
            .iter()
            .map(|(name, info)| {
                let mut command = CommandBuilder::new(
                    name,
                    "No description provided",
                    twilight_model::application::command::CommandType::ChatInput,
                );
                for option in &info.1 {
                    command = command.option(option.clone());
                }
                command.build()
            })
            .collect()
    }
}
