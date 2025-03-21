use std::fmt::Display;

use proc_macro2::Span;
use quote::{ToTokens, TokenStreamExt};
use syn::{parse::Parse, Error, Ident, Path};

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum PluginStage {
    Build,
    Finish,
    Cleanup,
}

impl PluginStage {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginStage::Build => "build",
            PluginStage::Finish => "finish",
            PluginStage::Cleanup => "cleanup",
        }
    }
}

impl From<PluginStage> for &'static str {
    fn from(value: PluginStage) -> Self {
        value.as_str()
    }
}

impl From<PluginStage> for usize {
    fn from(value: PluginStage) -> Self {
        match value {
            PluginStage::Build => 0,
            PluginStage::Cleanup => 1,
            PluginStage::Finish => 2,
        }
    }
}

impl Display for PluginStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(<Self as Into<&'static str>>::into(*self))
    }
}

impl TryFrom<&Ident> for PluginStage {
    type Error = Error;

    fn try_from(value: &Ident) -> Result<Self, Self::Error> {
        match value {
            value if value == "build" => Ok(PluginStage::Build),
            value if value == "finish" => Ok(PluginStage::Finish),
            value if value == "cleanup" => Ok(PluginStage::Cleanup),
            _ => Err(Error::new_spanned(
                value,
                format!("Unknown plugin stage \"{value}\""),
            )),
        }
    }
}

impl TryFrom<Ident> for PluginStage {
    type Error = Error;

    fn try_from(value: Ident) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&Path> for PluginStage {
    type Error = Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        value
            .require_ident()
            .and_then(Self::try_from)
    }
}

impl TryFrom<Path> for PluginStage {
    type Error = Error;

    fn try_from(value: Path) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl ToTokens for PluginStage {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let ident = Ident::new(self.as_str(), Span::call_site());
        tokens.append(ident);
    }
}

impl Parse for PluginStage {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Self::try_from(input.parse::<Ident>()?)
    }
}
