use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse::{Parse, ParseStream}, parse_macro_input, Error, Expr, ExprPath, Item, ItemFn, Meta, Path, Token};
use proc_macro_crate::{crate_name, FoundCrate};

mod butler_plugin_impl;
mod utils;

#[proc_macro_attribute]
pub fn butler_plugin(args: TokenStream, item: TokenStream) -> TokenStream
{
    let parsed: Item = parse_macro_input!(item as Item);

    match parsed {
        Item::Impl(item_impl) => butler_plugin_impl::butler_plugin_impl(args, item_impl),
        Item::Struct(item_struct) => butler_plugin_impl::butler_plugin_struct(args, item_struct),
        
        _ => Error::new_spanned(
            parsed,
            "#[butler_plugin] can only be invoked on structs or `impl Plugin` blocks."
        )
            .to_compile_error()
            .into()
    }
}

struct SystemArgs {
    schedule: ExprPath,
    plugin: Option<ExprPath>,
    transforms: Option<Expr>,
}

impl Parse for SystemArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut schedule: Option<ExprPath> = None;
        let mut plugin: Option<ExprPath> = None;
        let mut transforms: Option<Expr> = None;

        loop {
            let meta = input.parse::<Meta>()?;
            let name_value = meta.require_name_value()?;
            match name_value.path.get_ident().ok_or(input.error("Expected a name-value identifier"))? {
                ident if ident == "schedule" => {
                    match &name_value.value {
                        Expr::Path(path) => schedule = Some(path.clone()),
                        _ => return Err(input.error("Expected a Schedule")),
                    }
                },
                ident if ident == "plugin" => {
                    match &name_value.value {
                        Expr::Path(path) => plugin = Some(path.clone()),
                        _ => return Err(input.error("Expected a Plugin")),
                    }
                },
                ident if ident == "transforms" => {
                    transforms = Some(name_value.value.clone());
                }
                _ => {
                    return Err(input.error(format!("Unknown attribute \"{}\"", &name_value.path.to_token_stream())));
                }
            }

            if input.is_empty() {
                break;
            }
            else {
                input.parse::<Token![,]>()?;
            }
        }

        let schedule = schedule.ok_or(input.error("#[system] requires a \"schedule\""))?;
        Ok(Self {
            schedule,
            plugin,
            transforms
        })
    }
}

#[proc_macro_attribute]
pub fn system(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let bevy_app = find_bevy_crate("app", "bevy_app");
    let bevy_butler = find_bevy_butler();

    let args = parse_macro_input!(attr as SystemArgs);
    let schedule = args.schedule;
    let dppath = format!("{}::BevyButlerPlugin", bevy_butler.to_token_stream());
    let default_plugin = syn::parse_str(&dppath).expect(&format!("Failed to find {dppath}"));
    let plugin = args.plugin.unwrap_or(default_plugin);
    let func_name = input.sig.ident.clone();
    let transformed_func = args.transforms
        .map(|transforms| quote! { #func_name.#transforms})
        .unwrap_or_else(|| func_name.clone().into_token_stream());

    let butler_func_name = format_ident!("_butler_{}", func_name);

    

    quote! {
        #input

        fn #butler_func_name (plugin: &#plugin, app: &mut #bevy_app::App) {
            app.add_systems(#schedule, #transformed_func);
        }

        #bevy_butler::__internal::inventory::submit! {
            #bevy_butler::__internal::ButlerFunc::new::<#plugin>(#butler_func_name)
        }
    }.into()
}

fn find_bevy_crate(supercrate: &str, subcrate: &str) -> syn::Path {
    crate_name("bevy").map(|found|
        match found {
            FoundCrate::Itself => syn::parse_str(&format!("crate::{}", supercrate)).expect("Failed to unwrap self"),
            proc_macro_crate::FoundCrate::Name(name) => {
                syn::parse_str(&format!("::{}::{}", name, supercrate)).expect(&format!("Failed to parse path for ::{}::{}", name, supercrate))
            }
        }
    ).unwrap_or_else(|_| {
        crate_name(subcrate).map(|found| {
            match found {
                FoundCrate::Itself => syn::parse_str("crate").unwrap(),
                FoundCrate::Name(name) => {
                    syn::parse_str(&format!("::{}", &name)).expect(&format!("Failed to parse path for ::{}", name))
                }
            }
        }).expect(&format!("Failed to find bevy::{} or {}", supercrate, subcrate))
    })
}

fn find_bevy_butler() -> syn::Path {
    return crate_name("bevy-butler").map(|found| {
        match found {
            FoundCrate::Itself => syn::parse_str("::bevy_butler").expect("Failed to refer to bevy-butler"),
            FoundCrate::Name(name) => {
                syn::parse_str(&format!("::{}", &name.trim())).unwrap()
            }
        }
    }).expect("Failed to find bevy_butler");
}

struct ConfigurePlugin {
    plugin: Path,
}

impl Parse for ConfigurePlugin {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self { plugin: input.parse()? })
    }
}

#[proc_macro_attribute]
pub fn configure_plugin(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let func_name = &input.sig.ident;

    let plugin = parse_macro_input!(attr as ConfigurePlugin).plugin;

    let bevy_butler = find_bevy_butler();

    quote! {
        #input

        #bevy_butler::__internal::inventory::submit! {
            #bevy_butler::__internal::ButlerFunc::new::<#plugin>(#func_name)
        }
    }.into()
}