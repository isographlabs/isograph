#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum DefinitionKeyword {
    Schema,
    Scalar,
    Type,
    Interface,
    Union,
    Enum,
    Input,
    Directive,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum ExtensionKeyword {
    Schema,
    Scalar,
    Type,
    Interface,
    Union,
    Enum,
    Input,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum BooleanOrNullKeyword {
    True,
    False,
    Null,
}
