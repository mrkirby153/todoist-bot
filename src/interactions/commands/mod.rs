use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use twilight_model::{
    application::interaction::{Interaction, InteractionData},
    http::interaction::InteractionResponse,
};

use crate::AppState;

pub trait Command: Send + Sync + 'static {
    fn options() -> Vec<CommandOption>;
    fn from_interaction_data(data: &InteractionData) -> Self;
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct CommandOption {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
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

        let fut = (self.handler)(command_data, Arc::clone(&interaction), state);
        Box::pin(fut)
    }
}

pub struct CommandExecutor<S>
where
    S: Send + Sync + 'static,
{
    commands: HashMap<String, Box<dyn AsyncHandler<S>>>,
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

        self.commands.insert(name.to_string(), Box::new(handler));
    }

    pub async fn execute(
        &self,
        name: &str,
        interaction: Arc<Interaction>,
        state: Arc<S>,
    ) -> Option<InteractionResponse> {
        let handler = self.commands.get(name)?;
        Some(handler.handle(interaction, state).await)
    }
}
