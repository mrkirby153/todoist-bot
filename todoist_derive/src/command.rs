use darling::FromDeriveInput;
use darling::FromField;
use darling::ast::Data;
use proc_macro::TokenStream;
use quote::quote;
use syn::Ident;
use syn::parse_macro_input;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(command), supports(struct_named))]
struct CommandReceiver {
    ident: syn::Ident,
    data: Data<(), OptionReceiver>,
    name: String,
    #[darling(default)]
    description: Option<String>,
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

    let options_raw: Result<_, darling::Error> = fields
        .iter()
        .map(|field| {
            // Assert that either field_name_override or field_name is Some
            let name = get_name(field)?;

            let default_description = "".to_string();
            let description = field.description.as_ref().unwrap_or(&default_description);
            let ty = &field.ty;
            let ty = if is_option_type(ty) {
                let inner_type = get_inner_option_type(ty).unwrap();
                quote! {
                    Option::<#inner_type>
                }
            } else {
                quote! {
                    #ty
                }
            };
            Ok(quote! {
                #ty::to_option().name(#name).description(#description)
            })
        })
        .collect();

    let options: Vec<proc_macro2::TokenStream> = match options_raw {
        Ok(tokens) => tokens,
        Err(err) => return TokenStream::from(err.write_errors()),
    };

    let field_names: Result<Vec<(String, Ident)>, darling::Error> = fields
        .iter()
        .map(|field| {
            let name = get_name(field)?;
            let ident = field.ident.as_ref().unwrap().clone();
            Ok((name, ident))
        })
        .collect();
    let field_names = match field_names {
        Ok(names) => names,
        Err(err) => return TokenStream::from(err.write_errors()),
    };

    let ident = receiver.ident;

    let struct_fields = field_names.iter().map(|(name, field_ident)| {
        quote! {
            #field_ident: crate::interactions::commands::arguments::parse(&options_map, #name)?
        }
    });

    let description = if let Some(desc) = &receiver.description {
        desc.as_str()
    } else {
        "No description provided"
    };

    let command_name = &receiver.name;

    quote! {
        #[automatically_derived]
        impl crate::interactions::commands::Command for #ident {
            fn options() -> Vec<crate::interactions::commands::arguments::CommandOption> {
                use crate::interactions::commands::arguments::ToOption;
                vec![
                    #(#options),*
                ]
            }
            fn name() -> &'static str {
                #command_name
            }
            fn description() -> &'static str {
                #description
            }
            fn from_interaction_data(data: &::twilight_model::application::interaction::InteractionData) -> Result<Self, crate::interactions::commands::arguments::Error> {
                if let ::twilight_model::application::interaction::InteractionData::ApplicationCommand(command_data) = data {
                    let options_map = command_data
                        .options
                        .iter()
                        .map(|opt| (opt.name.clone(), opt.value.clone()))
                        .collect::<::std::collections::HashMap<_, _>>();

                    Ok(Self {
                        #(#struct_fields,)*
                    })
                } else {
                    Err(crate::interactions::commands::arguments::Error::InvalidType)
                }
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

fn get_inner_option_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(angle_bracketed) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner_type)) = angle_bracketed.args.first()
    {
        return Some(inner_type);
    }
    None
}

fn get_name(field: &OptionReceiver) -> Result<String, darling::Error> {
    if let Some(name) = &field.name {
        Ok(name.clone())
    } else if let Some(ident) = &field.ident {
        Ok(ident.to_string())
    } else {
        Err(darling::Error::custom("Field must have a name"))
    }
}
