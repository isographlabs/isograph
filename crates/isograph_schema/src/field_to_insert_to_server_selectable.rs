use common_lang_types::WithLocationPostfix;
use graphql_lang_types::{GraphQLConstantValue, GraphQLNamedTypeAnnotation, NameValuePair};
use isograph_lang_types::ConstantValue;

use crate::FlattenedDataModelSelectable;

pub type ScalarSelectionAndNonNullType<TCompilationProfile> = (
    FlattenedDataModelSelectable<TCompilationProfile>,
    Option<GraphQLNamedTypeAnnotation>,
);

pub fn to_isograph_constant_value(graphql_constant_value: GraphQLConstantValue) -> ConstantValue {
    match graphql_constant_value {
        GraphQLConstantValue::Int(i) => ConstantValue::Integer(i),
        GraphQLConstantValue::Boolean(b) => ConstantValue::Boolean(b),
        GraphQLConstantValue::String(s) => ConstantValue::String(s),
        GraphQLConstantValue::Float(f) => ConstantValue::Float(f),
        GraphQLConstantValue::Null => ConstantValue::Null,
        GraphQLConstantValue::Enum(e) => ConstantValue::Enum(e),
        GraphQLConstantValue::List(l) => {
            let converted_list = l
                .into_iter()
                .map(|x| to_isograph_constant_value(x.item).with_location(x.location))
                .collect::<Vec<_>>();
            ConstantValue::List(converted_list)
        }
        GraphQLConstantValue::Object(o) => {
            let converted_object = o
                .into_iter()
                .map(|name_value_pair| NameValuePair {
                    name: name_value_pair.name,
                    value: to_isograph_constant_value(name_value_pair.value.item)
                        .with_location(name_value_pair.value.location),
                })
                .collect::<Vec<_>>();
            ConstantValue::Object(converted_object)
        }
    }
}
