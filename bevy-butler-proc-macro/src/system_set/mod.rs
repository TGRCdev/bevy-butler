use proc_macro::TokenStream as TokenStream1;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use structs::SystemSetInput;
use syn::{parse::{Parse, Parser}, punctuated::Punctuated, spanned::Spanned, Error, Expr, Item, Token};

use crate::system::structs::SystemAttr;

pub mod structs;

pub(crate) fn macro_impl(body: TokenStream1) -> syn::Result<TokenStream2> {
    let input = SystemSetInput::parse.parse(body)?;
    let set_args = input.system_args;
    let mut items = input.items;

    // We'll wrap every system into a single set, so no overriding
    // the plugin/schedule
    let plugin = set_args.require_plugin()?;
    let schedule = set_args.require_schedule()?;
    let set_transforms = set_args.transforms.iter();
    if set_args.generics.is_some() {
        return Err(Error::new(set_args.attr_span, "`generics` is not applicable for `system_set!` arguments"));
    }

    let mut systems: Punctuated<Expr, Token![,]> = Default::default();

    // We are going to manually parse every system and handle its
    // attribute ourselves, instead of letting #[system] register
    // it. Then we'll create a single registry entry to register our
    // transformed set.
    for item in items.iter_mut() {
        match item {
            Item::Fn(item_fn) => {
                // extract_if is still unstable, so we gotta get a little icky with the code
                {
                    let mut i = 0;
                    let attrs = &mut item_fn.attrs;
                    while i < attrs.len() {
                        if attrs[i].path().get_ident().is_some_and(|ident| ident == "system") {
                            
                            let sys_attr = attrs.remove(i);
                            let sys_args = if matches!(sys_attr.meta, syn::Meta::Path(_)) {
                                SystemAttr::default()
                            } else {
                                sys_attr.parse_args()?
                            };
                            if sys_args.plugin.is_some() {
                                return Err(Error::new(sys_attr.span(), "`plugin` can not be overridden within a `system_set!` block"));
                            }
                            if sys_args.schedule.is_some() {
                                return Err(Error::new(sys_attr.span(), "`schedule` can not be overridden within a `system_set!` block"));
                            }
                            let fn_ident = &item_fn.sig.ident;
                            let generics = sys_args.generics;
                            let transforms = sys_args.transforms.into_iter();
                            systems.push(syn::parse2(quote!(#fn_ident #generics #(. #transforms)*))?);
                        }
                        else {
                            i += 1;
                        }
                    }
                }
            }
            _ => (),
        }
    }

    // Construct the system set as an Expr
    let system_set: Expr = syn::parse2(quote!( (#systems) #(. #set_transforms)* ))?;

    // Hash the system set to get a static ident
    #[allow(unused)]
    let set_hash = sha256::digest(format!("{}{}{}",
        plugin.to_token_stream().to_string(),
        schedule.to_token_stream().to_string(),
        system_set.clone().to_token_stream().to_string(),
    ));

    let static_ident = format_ident!("_butler_sys_set_{}", set_hash);

    let register_block = quote! {
        ::bevy_butler::butler_entry!(#static_ident, ::bevy_butler::__internal::ButlerRegistryEntryFactory::new(
            || #plugin::_butler_sealed_marker(),
            |app| { app.add_systems( #schedule, #system_set ); }
        ));
    };

    Ok(quote! {
        #(#items)*

        #register_block
    })
}