use darling::FromDeriveInput;
use darling::FromField;
use darling::ast::Data;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(command), supports(struct_named))]
struct CommandReceiver {
    ident: syn::Ident,
    data: Data<(), OptionReceiver>,
}

#[derive(Debug, FromField)]
#[darling(attributes(option))]
struct OptionReceiver {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(default)]
    name: Option<String>,
    #[darling(default)]
    description: Option<String>,
}

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let receiver = match CommandReceiver::from_derive_input(&input) {
        Ok(val) => val,
        Err(err) => return TokenStream::from(err.write_errors()),
    };

    let fields = receiver.data.take_struct().unwrap().fields;

    let fields = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref();
            let field_name_override = field.name.as_ref();

            // Assert that either field_name_override or field_name is Some
            let name = if let Some(name) = field_name_override {
                name.clone()
            } else if let Some(ident) = field_name {
                ident.to_string()
            } else {
                return Err(darling::Error::custom("Field must have a name"));
            };

            let default_description = "".to_string();
            let description = field.description.as_ref().unwrap_or(&default_description);
            let required = !is_option_type(&field.ty);

            Ok(quote! {
                crate::interactions::commands::CommandOption {
                    name: stringify!(#name),
                    description: stringify!(#description),
                    required: #required,
                }
            })
        })
        .collect();

    let fields: Vec<proc_macro2::TokenStream> = match fields {
        Ok(tokens) => tokens,
        Err(err) => return TokenStream::from(err.write_errors()),
    };

    let ident = receiver.ident;

    quote! {
        #[automatically_derived]
        impl crate::interactions::commands::Command for #ident {
            fn options() -> Vec<crate::interactions::commands::CommandOption> {
                vec![
                    #(#fields),*
                ]
            }
            fn from_interaction_data(data: &::twilight_model::application::interaction::InteractionData) -> Self {
                todo!("Not yet implemented")
            }
        }
    }
    .into()
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}
