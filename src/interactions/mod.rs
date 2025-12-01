use tracing::debug;

use twilight_model::application::interaction::application_command::{
    CommandData, CommandDataOption, CommandOptionValue,
};

pub mod command_handlers;
pub mod verifier;

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
