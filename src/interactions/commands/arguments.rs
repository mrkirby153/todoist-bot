use std::collections::HashMap;

use twilight_model::application::interaction::application_command::CommandOptionValue;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid type for command option")]
    InvalidType,
    #[error("Missing field for command option")]
    MissingField,
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

pub fn parse<T: OptionalArgumentConverter>(
    options: &HashMap<String, CommandOptionValue>,
    name: &str,
) -> Result<T, T::Error> {
    T::convert(options.get(name))
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
