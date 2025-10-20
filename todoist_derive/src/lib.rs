use proc_macro::TokenStream;

mod command;

#[proc_macro_derive(Command, attributes(option, command))]
pub fn command_derive(input: TokenStream) -> TokenStream {
    command::derive(input)
}
