use deluxe::ParseMetaItem;
use syn::{AngleBracketedGenericArguments, Path};

#[derive(ParseMetaItem)]
pub(crate) struct ObserverAttr {
    pub plugin: Path,
    pub generics: Option<AngleBracketedGenericArguments>,
}
