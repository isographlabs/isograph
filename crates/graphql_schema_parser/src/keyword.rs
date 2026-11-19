#[cfg_attr(not(test), expect(dead_code))]
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

#[expect(dead_code)]
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

#[expect(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum BooleanOrNullKeyword {
    True,
    False,
    Null,
}
