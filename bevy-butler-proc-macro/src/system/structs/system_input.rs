use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{parse::{discouraged::Speculative, Parse, ParseStream, Parser}, punctuated::Punctuated, AngleBracketedGenericArguments, Error, ExprCall, GenericArgument, Ident, ItemFn, Meta, MetaList, MetaNameValue, Token, TypePath};

use crate::config_systems::CONFIG_SYSTEMS_DEFAULT_ARGS_IDENT;

#[derive(Default, Debug, Clone)]
pub(crate) struct SystemAttr {
    pub plugin: Option<TypePath>,
    pub schedule: Option<TypePath>,
    pub generics: Option<AngleBracketedGenericArguments>,
    pub transforms: Punctuated<ExprCall, Token![.]>,
}

impl SystemAttr {
    pub fn require_plugin(&self) -> syn::Result<&TypePath> {
        self.plugin.as_ref().ok_or(Error::new(Span::call_site(), "#[system] requires a defined or inherited `plugin` argument"))
    }

    pub fn require_schedule(&self) -> syn::Result<&TypePath> {
        self.schedule.as_ref().ok_or(Error::new(Span::call_site(), "#[system] requires a defined or inherited `schedule` argument"))
    }

    /// Override the arguments on this SystemAttr with `overlay`'s arguments,
    /// if present.
    pub fn overlay(&mut self, overlay: Self) {
        self.generics = overlay.generics.or(self.generics.take());
        self.schedule = overlay.schedule.or(self.schedule.take());
        self.plugin = overlay.plugin.or(self.plugin.take());

        // Append the overlay transforms to the end
        self.transforms.extend(overlay.transforms);
    }

    pub fn with_defaults(&mut self, defaults: Self) {
        self.generics = self.generics.take().or(defaults.generics);
        self.schedule = self.schedule.take().or(defaults.schedule);
        self.plugin = self.plugin.take().or(defaults.plugin);

        // Append our transforms onto the end of the defaults
        let mut transforms = defaults.transforms;
        transforms.extend(std::mem::take(&mut self.transforms));
        self.transforms = transforms;
    }

    fn parse_type_path_meta(meta: Meta) -> syn::Result<TypePath> {
        match meta {
            Meta::List(list) => Ok(syn::parse2(list.tokens)?),
            Meta::NameValue(name_value) => Ok(syn::parse2(name_value.value.to_token_stream())?),
            Meta::Path(p) => Err(Error::new_spanned(p, "Expected name-value pair or list containing a TypePath")),
        }
    }

    pub fn insert_generics(&mut self, mut generics: AngleBracketedGenericArguments) -> syn::Result<&mut AngleBracketedGenericArguments> {
        if self.generics.is_some() {
            return Err(Error::new_spanned(generics, "Multiple declarations of \"generics\""));
        }

        generics.colon2_token = Some(Default::default());

        Ok(self.generics.insert(generics))
    }

    pub fn parse_plugin_meta(&mut self, meta: Meta) -> syn::Result<&mut TypePath> {
        if self.plugin.is_some() {
            return Err(Error::new_spanned(meta, "Multiple declarations of \"plugin\""));
        }

        Ok(self.plugin.insert(Self::parse_type_path_meta(meta)?))
    }

    pub fn parse_schedule_meta(&mut self, meta: Meta) -> syn::Result<&mut TypePath> {
        if self.schedule.is_some() {
            return Err(Error::new_spanned(meta, "Multiple declarations of \"schedule\""));
        }

        Ok(self.schedule.insert(Self::parse_type_path_meta(meta)?))
    }

    pub fn parse_transform_meta(&mut self, meta: Meta) -> syn::Result<&mut Punctuated<ExprCall, Token![.]>> {
        let expr: ExprCall = match meta {
            // No-argument transform like `chain()`
            Meta::Path(path) => syn::parse2(quote!(#path ()))?,

            // Single argument transform like `run_if(some_condition)`
            Meta::NameValue(MetaNameValue { path, value, .. }) => syn::parse2(quote!(#path (#value)))?,

            // Multiple argument transform (currently doesn't exist within Bevy but may be a user-defined transform)
            Meta::List(MetaList { path, tokens, ..}) => syn::parse2(quote!(#path (#tokens)))?,
        };

        self.transforms.push(expr);
        Ok(&mut self.transforms)
    }

    pub fn parse_generics_meta(&mut self, meta: Meta) -> syn::Result<&mut AngleBracketedGenericArguments> {
        let mut generics = AngleBracketedGenericArguments {
            colon2_token: Some(Default::default()),
            lt_token: Default::default(),
            gt_token: Default::default(),
            args: Default::default(),
        };

        match meta {
            Meta::List(list) => generics.args = list.parse_args_with(Punctuated::<GenericArgument, Token![,]>::parse_terminated)?,
            Meta::NameValue(name_value) => generics.args = Punctuated::<GenericArgument, Token![,]>::parse_terminated.parse2(name_value.value.to_token_stream())?,
            Meta::Path(p) => return Err(Error::new_spanned(p, "Expected name-value pair or list containing generic arguments")),
        }

        Ok(self.insert_generics(generics)?)
    }

    pub fn parse_meta(&mut self, meta: Meta) -> syn::Result<()> {
        match meta.path().get_ident() {
            Some(ident) if ident == "plugin" => { self.parse_plugin_meta(meta)?; }
            Some(ident) if ident == "schedule" => { self.parse_schedule_meta(meta)?; }
            Some(ident) if ident == "generics" => { self.parse_generics_meta(meta)?; }
            Some(_) | None => { self.parse_transform_meta(meta)?; }
        }

        Ok(())
    }

    pub fn get_metas(&self) -> Punctuated<Meta, Token![,]> {
        let mut args = Punctuated::<Meta, Token![,]>::new();
        if let Some(meta) = self.plugin.as_ref().map(|plugin| syn::parse_quote!(plugin = #plugin)) {
            args.push(meta);
        }
        if let Some(meta) = self.schedule.as_ref().map(|schedule| syn::parse_quote!(schedule = #schedule)) {
            args.push(meta);
        }
        if let Some(meta) = self.generics.as_ref().map(|generics| {
            let generics = &generics.args;
            syn::parse_quote!(generics(#generics))
        }) {
            args.push(meta);
        }
        self.transforms.iter().for_each(|trns| {
            args.push(syn::parse_quote!(#trns));
        });

        args
    }
}

impl Parse for SystemAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut ret = SystemAttr {
            plugin: None,
            schedule: None,
            generics: None,
            transforms: Default::default(),
        };
        // We are in a list (a = ..., b(c), ...)

        // Do some speculative parsing for `generics = <...>` because
        // syn doesn't like angle brackets in Meta
        while !input.is_empty() {
            // Fork and try to parse a Meta first
            let fork = input.fork();
            match fork.parse::<Meta>() {
                Ok(meta) => {
                    input.advance_to(&fork);
                    ret.parse_meta(meta)?;
                }
                Err(e) => {
                    // Try to parse `generics = <TypePath>`, otherwise just return the error
                    if input.parse::<Ident>().map_err(|_| e.clone())? != "generics" {
                        return Err(e);
                    }
                    input.parse::<Token![=]>().map_err(|_| e.clone())?;
                    ret.insert_generics(AngleBracketedGenericArguments::parse(input)?)?;
                }
            }
            if input.peek(Token![,])
                { input.parse::<Token![,]>()?; }
        }

        Ok(ret)
    }
}

pub(crate) struct SystemInput {
    pub attr: SystemAttr,
    pub body: ItemFn,
}

impl SystemInput {
    pub fn parse_with_attr(mut attr: SystemAttr) -> impl Parser<Output = Self> {
        |input: ParseStream| {
            let body: ItemFn = input.parse()?;

            let mut default_args = SystemAttr::default();

            // Check for default args provided by config_systems
            for attr in &body.attrs {
                if attr.path().segments.last().is_some_and(|seg| seg.ident == CONFIG_SYSTEMS_DEFAULT_ARGS_IDENT) {
                    default_args.overlay(attr.parse_args()?);
                }
            }

            attr.with_defaults(default_args);

            Ok(Self {
                attr,
                body,
            })
        }
    }
}