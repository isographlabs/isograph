use common_lang_types::StringLiteralValue;
use intern::{Lookup, string_key::Intern};

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[serde(deny_unknown_fields)]
pub struct FieldMapItem {
    // TODO eventually, we want to support . syntax here, too
    pub from: StringLiteralValue,
    pub to: StringLiteralValue,
}

pub struct SplitToArg {
    pub to_argument_name: StringLiteralValue,
    pub to_field_names: Vec<StringLiteralValue>,
}

impl FieldMapItem {
    pub fn split_to_arg(&self) -> SplitToArg {
        let mut split = self.to.lookup().split('.');
        let to_argument_name = split.next().expect(
            "Expected at least one item returned \
                by split. This is indicative of a bug in Isograph.",
        );

        SplitToArg {
            to_argument_name: to_argument_name.intern().into(),
            to_field_names: split.map(|x| x.intern().into()).collect(),
        }
    }
}
