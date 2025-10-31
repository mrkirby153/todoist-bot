use core::panic;
use std::fmt::Debug;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use crate::interactions::commands::arguments::CommandOption;
use tracing::debug;
use twilight_model::application::command::Command as TwilightCommand;
use twilight_model::application::interaction::InteractionContextType;
use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};
use twilight_model::channel::message::MessageFlags;
use twilight_model::{
    application::interaction::Interaction,
    http::interaction::{InteractionResponse, InteractionResponseData},
};
use twilight_util::builder::command::{CommandBuilder, SubCommandBuilder, SubCommandGroupBuilder};
use twilight_util::builder::message::{ContainerBuilder, TextDisplayBuilder};
pub mod arguments;
use anyhow::Error;

use twilight_model::http::interaction::InteractionResponseType;
pub trait Command: Send + Sync + 'static + Sized {
    fn options() -> Vec<CommandOption>;
    fn from_command_data(data: Vec<CommandDataOption>) -> Result<Self, arguments::Error>;
    fn description() -> &'static str;
    fn name() -> &'static str;
}

type CommandResponse = Result<InteractionResponse, Error>;

// Trait for type-erased async handlers
trait AsyncHandler<S>: Send + Sync {
    fn handle(
        &self,
        interaction: Arc<Interaction>,
        interaction_data: Vec<CommandDataOption>,
        state: Arc<S>,
    ) -> Pin<Box<dyn Future<Output = CommandResponse> + Send>>;
}

// Concrete implementation that preserves command type
struct TypedAsyncHandler<C, S, F, Fut>
where
    C: Command,
    F: Fn(C, Arc<Interaction>, Arc<S>) -> Fut + Send + Sync,
    Fut: Future<Output = CommandResponse> + Send + 'static,
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
    Fut: Future<Output = CommandResponse> + Send + 'static,
{
    fn handle(
        &self,
        interaction: Arc<Interaction>,
        options: Vec<CommandDataOption>,
        state: Arc<S>,
    ) -> Pin<Box<dyn Future<Output = CommandResponse> + Send>> {
        let command_data = C::from_command_data(options);

        let command_data = match command_data {
            Ok(data) => data,
            Err(_) => {
                return Box::pin(async {
                    Ok(InteractionResponse {
                        kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData {
                            content: Some("Failed to parse command data.".to_string()),
                            flags: Some(MessageFlags::EPHEMERAL),
                            ..Default::default()
                        }),
                    })
                });
            }
        };

        let fut = (self.handler)(command_data, Arc::clone(&interaction), state);
        Box::pin(fut)
    }
}

struct CommandInfo<S>(Box<dyn AsyncHandler<S>>, Vec<CommandOption>, &'static str)
where
    S: Send + Sync + 'static;

enum CommandTree<S>
where
    S: Send + Sync + 'static,
{
    Node(HashMap<String, CommandTree<S>>),
    Leaf(CommandInfo<S>),
}

impl<S> CommandTree<S>
where
    S: Send + Sync + 'static,
{
    fn new() -> Self {
        CommandTree::Node(HashMap::new())
    }

    fn insert(&mut self, path: &[String], info: CommandInfo<S>) {
        match self {
            CommandTree::Node(children) => {
                if path.is_empty() {
                    return;
                }
                let key = &path[0];
                if path.len() == 1 {
                    children.insert(key.clone(), CommandTree::Leaf(info));
                } else {
                    let child = children.entry(key.clone()).or_insert_with(CommandTree::new);
                    child.insert(&path[1..], info);
                }
            }
            CommandTree::Leaf(_) => {
                panic!("Cannot insert into a leaf node");
            }
        }
    }

    fn get(&self, path: &[String]) -> Option<&CommandInfo<S>> {
        match self {
            CommandTree::Node(children) => {
                if path.is_empty() {
                    return None;
                }
                let key = &path[0];
                let child = children.get(key)?;
                if path.len() == 1 {
                    match child {
                        CommandTree::Leaf(info) => Some(info),
                        CommandTree::Node(_) => None,
                    }
                } else {
                    child.get(&path[1..])
                }
            }
            CommandTree::Leaf(_) => None,
        }
    }
}

impl<S> Debug for CommandTree<S>
where
    S: Send + Sync + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandTree::Node(children) => {
                write!(f, "Node {{ ")?;
                for (key, child) in children {
                    write!(f, "{}: {:?}, ", key, child)?;
                }
                write!(f, "}}")
            }
            CommandTree::Leaf(_) => write!(f, "Leaf"),
        }
    }
}

pub struct CommandExecutor<S>
where
    S: Send + Sync + 'static,
{
    commands: CommandTree<S>,
}

impl<S> CommandExecutor<S>
where
    S: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            commands: CommandTree::new(),
        }
    }

    /// Register an async command handler
    pub fn register<C, F, Fut>(&mut self, handler: F)
    where
        C: Command,
        F: Fn(C, Arc<Interaction>, Arc<S>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CommandResponse> + Send + 'static,
    {
        let handler = TypedAsyncHandler {
            handler,
            _phantom: std::marker::PhantomData,
        };

        let name = C::name().to_string();
        let command_info = CommandInfo(Box::new(handler), C::options(), C::description());

        let path = name.split(' ').map(String::from).collect::<Vec<_>>();
        self.commands.insert(&path, command_info);
    }

    pub async fn execute(
        &self,
        name: &str,
        interaction: Arc<Interaction>,
        options: Vec<CommandDataOption>,
        state: Arc<S>,
    ) -> Option<InteractionResponse> {
        let path = name.split(' ').map(String::from).collect::<Vec<_>>();
        let handler = self.commands.get(&path)?;

        Some(
            handler
                .0
                .handle(interaction, options, state)
                .await
                .unwrap_or_else(|e| {
                    let container = ContainerBuilder::new()
                        .accent_color(Some(0xAA0000))
                        .component(
                            TextDisplayBuilder::new(format!("An error occurred: {}", e)).build(),
                        )
                        .build();

                    InteractionResponse {
                        kind: InteractionResponseType::ChannelMessageWithSource,
                        data: Some(InteractionResponseData {
                            components: Some(vec![container.into()]),
                            flags: Some(MessageFlags::EPHEMERAL | MessageFlags::IS_COMPONENTS_V2),
                            ..Default::default()
                        }),
                    }
                }),
        )
    }

    pub fn build_commands(&self) -> Vec<TwilightCommand> {
        let mut commands: Vec<TwilightCommand> = Vec::new();

        if let CommandTree::Node(children) = &self.commands {
            for (name, child) in children.iter() {
                let mut command;

                match child {
                    CommandTree::Leaf(info) => {
                        // This is a top-level command
                        command = CommandBuilder::new(
                            name,
                            info.2,
                            twilight_model::application::command::CommandType::ChatInput,
                        )
                        .contexts(vec![
                            InteractionContextType::Guild,
                            InteractionContextType::BotDm,
                            InteractionContextType::PrivateChannel,
                        ]);
                        for option in &info.1 {
                            command = command.option(option.clone());
                        }
                    }
                    CommandTree::Node(subcommand_or_group) => {
                        command = CommandBuilder::new(
                            name,
                            "No description provided",
                            twilight_model::application::command::CommandType::ChatInput,
                        )
                        .contexts(vec![
                            InteractionContextType::Guild,
                            InteractionContextType::BotDm,
                            InteractionContextType::PrivateChannel,
                        ]);
                        for (grandchild_name, grandchild) in subcommand_or_group.iter() {
                            match grandchild {
                                CommandTree::Leaf(info) => {
                                    // This is a subcommand
                                    let mut subcommand =
                                        SubCommandBuilder::new(grandchild_name, info.2);
                                    for option in &info.1 {
                                        subcommand = subcommand.option(option.clone());
                                    }
                                    command = command.option(subcommand.build());
                                }
                                CommandTree::Node(_) => {
                                    // This is a subcommand group
                                    if let CommandTree::Node(sub_subcommands) = grandchild {
                                        let subcommand_group = SubCommandGroupBuilder::new(
                                            grandchild_name,
                                            "No description provided",
                                        );
                                        let mut subcommands = Vec::new();

                                        for (subchild_name, subchild) in sub_subcommands.iter() {
                                            if let CommandTree::Leaf(info) = subchild {
                                                let mut subcommand =
                                                    SubCommandBuilder::new(subchild_name, info.2);
                                                for option in &info.1 {
                                                    subcommand = subcommand.option(option.clone());
                                                }
                                                subcommands.push(subcommand);
                                            }
                                        }
                                        command = command.option(
                                            subcommand_group.subcommands(subcommands).build(),
                                        );
                                    } else {
                                        panic!("Expected Node for subcommand group");
                                    }
                                }
                            }
                        }
                    }
                }
                commands.push(command.build());
            }

            commands
        } else {
            panic!("Root of command tree must be a node");
        }
    }
}

impl<S> Default for CommandExecutor<S>
where
    S: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

pub fn resolve_command_path(interaction: &CommandData) -> Option<(String, Vec<CommandDataOption>)> {
    debug!("Resolving command path for interaction: {:?}", interaction);
    let mut path = vec![interaction.name.clone()];

    if !is_option_sub(&interaction.options) {
        return Some((path.join(" "), interaction.options.clone()));
    }

    let option = &interaction.options[0];
    match &option.value {
        CommandOptionValue::SubCommand(options) => {
            path.push(option.name.clone());
            Some((path.join(" "), options.clone()))
        }
        CommandOptionValue::SubCommandGroup(group) => {
            let group_name = &option.name;
            path.push(group_name.clone());
            let subcommand = &group[0];
            path.push(subcommand.name.clone());
            if let CommandOptionValue::SubCommand(options) = &subcommand.value {
                Some((path.join(" "), options.clone()))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_option_sub(options: &[CommandDataOption]) -> bool {
    if options.is_empty() {
        return false;
    }
    matches!(
        &options[0].value,
        CommandOptionValue::SubCommand(_) | CommandOptionValue::SubCommandGroup(_)
    )
}
