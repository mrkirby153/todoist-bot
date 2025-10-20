#![allow(
    dead_code,
    reason = "Traits and types are implemented by todoist_derive"
)]
use std::collections::HashMap;

use twilight_model::{
    application::{
        command::{
            CommandOptionChoice, CommandOptionType,
            CommandOptionValue as InteractionCommandOptionValue,
        },
        interaction::application_command::CommandOptionValue,
    },
    channel::ChannelType,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid type for command option")]
    InvalidType,
    #[error("Missing field for command option")]
    MissingField,
}

#[derive(Debug, Clone)]
pub struct CommandOption {
    pub autocomplete: Option<bool>,
    pub channel_types: Option<Vec<ChannelType>>,
    pub choices: Option<Vec<CommandOptionChoice>>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub kind: CommandOptionType,
    pub max_length: Option<u16>,
    pub max_value: Option<InteractionCommandOptionValue>,
    pub min_length: Option<u16>,
    pub min_value: Option<InteractionCommandOptionValue>,
    pub required: Option<bool>,
}

pub trait ToOption {
    fn to_option() -> CommandOption;
}

pub trait OptionalArgumentConverter: Sized {
    type Error;

    fn convert(data: Option<&CommandOptionValue>) -> Result<Self, Self::Error>;
}

pub trait ArgumentConverter: Sized {
    type Error;

    fn convert(data: &CommandOptionValue) -> Result<Self, Self::Error>;
}

impl<T: OptionalArgumentConverter<Error = Error>> OptionalArgumentConverter for Option<T> {
    type Error = Error;

    fn convert(data: Option<&CommandOptionValue>) -> Result<Self, Self::Error> {
        match data {
            Some(_) => Ok(Some(T::convert(data)?)),
            None => Ok(None),
        }
    }
}

impl<T: ArgumentConverter<Error = Error>> OptionalArgumentConverter for T {
    type Error = T::Error;

    fn convert(data: Option<&CommandOptionValue>) -> Result<Self, Self::Error> {
        if let Some(value) = data {
            T::convert(value)
        } else {
            Err(Error::MissingField)
        }
    }
}

impl CommandOption {
    pub fn new(kind: CommandOptionType) -> Self {
        CommandOption {
            autocomplete: None,
            channel_types: None,
            choices: None,
            name: None,
            description: None,
            kind,
            max_length: None,
            max_value: None,
            min_length: None,
            min_value: None,
            required: None,
        }
    }

    pub fn autocomplete(mut self, autocomplete: bool) -> Self {
        self.autocomplete = Some(autocomplete);
        self
    }

    pub fn channel_types(mut self, channel_types: Vec<ChannelType>) -> Self {
        self.channel_types = Some(channel_types);
        self
    }

    pub fn choices(mut self, choices: Vec<CommandOptionChoice>) -> Self {
        self.choices = Some(choices);
        self
    }

    pub fn max_length(mut self, max_length: u16) -> Self {
        self.max_length = Some(max_length);
        self
    }

    pub fn max_value(mut self, max_value: InteractionCommandOptionValue) -> Self {
        self.max_value = Some(max_value);
        self
    }

    pub fn min_length(mut self, min_length: u16) -> Self {
        self.min_length = Some(min_length);
        self
    }

    pub fn min_value(mut self, min_value: InteractionCommandOptionValue) -> Self {
        self.min_value = Some(min_value);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
}

pub fn parse<T: OptionalArgumentConverter>(
    options: &HashMap<String, CommandOptionValue>,
    name: &str,
) -> Result<T, T::Error> {
    T::convert(options.get(name))
}

impl<T: ToOption> ToOption for Option<T> {
    fn to_option() -> CommandOption {
        T::to_option().required(false)
    }
}

impl From<CommandOption> for twilight_model::application::command::CommandOption {
    fn from(option: CommandOption) -> Self {
        twilight_model::application::command::CommandOption {
            autocomplete: option.autocomplete,
            channel_types: option.channel_types,
            choices: option.choices,
            name: option.name.unwrap_or_default(),
            description: option.description.unwrap_or_default(),
            kind: option.kind,
            max_length: option.max_length,
            max_value: option.max_value,
            min_length: option.min_length,
            min_value: option.min_value,
            required: option.required,
            description_localizations: None,
            name_localizations: None,
            options: None,
        }
    }
}

// --- Implementations for common types ---
impl ArgumentConverter for String {
    type Error = Error;

    fn convert(data: &CommandOptionValue) -> Result<Self, Self::Error> {
        if let CommandOptionValue::String(value) = data {
            Ok(value.clone())
        } else {
            Err(Error::InvalidType)
        }
    }
}

impl ToOption for String {
    fn to_option() -> CommandOption {
        CommandOption::new(CommandOptionType::String)
    }
}

impl ArgumentConverter for i32 {
    type Error = Error;

    fn convert(data: &CommandOptionValue) -> Result<Self, Self::Error> {
        if let CommandOptionValue::Integer(value) = data {
            Ok(*value as i32)
        } else {
            Err(Error::InvalidType)
        }
    }
}

impl ToOption for i32 {
    fn to_option() -> CommandOption {
        CommandOption::new(CommandOptionType::Integer)
    }
}
