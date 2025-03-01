use common_lang_types::DescriptionValue;
use isograph_lang_types::{DefinitionLocation, SelectionType, ServerObjectId, TypeAnnotation};

use crate::{
    ClientPointer, OutputFormat, ServerObjectField, ValidatedClientPointer,
    ValidatedSchemaServerField, ValidatedServerObjectField,
};

#[allow(clippy::type_complexity)]
pub fn description<
    ServerFieldTypeAssociatedData,
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
    TSelectionTypeVariableDefinitionAssociatedData: Ord + std::fmt::Debug,
    TOutputFormat: OutputFormat,
>(
    definition_location: &DefinitionLocation<
        &ServerObjectField<
            ServerFieldTypeAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
        &ClientPointer<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
            TSelectionTypeVariableDefinitionAssociatedData,
            TOutputFormat,
        >,
    >,
) -> Option<DescriptionValue> {
    match definition_location {
        DefinitionLocation::Server(server_field) => server_field.description,
        DefinitionLocation::Client(client_field) => client_field.description,
    }
}

pub fn output_type_annotation<TOutputFormat: OutputFormat>(
    definition_location: &DefinitionLocation<
        &ValidatedServerObjectField<TOutputFormat>,
        &ValidatedClientPointer<TOutputFormat>,
    >,
) -> TypeAnnotation<ServerObjectId> {
    match definition_location {
        DefinitionLocation::Client(client_pointer) => client_pointer.to.clone(),
        DefinitionLocation::Server(server_field) => match &server_field.associated_data {
            SelectionType::Scalar(_) => panic!(
                "output_type_id should be an object. \
                    This is indicative of a bug in Isograph.",
            ),
            SelectionType::Object(associated_data) => associated_data.type_name.clone(),
        },
    }
}

pub fn as_server_field<TFieldAssociatedData, TClientFieldType>(
    definition_location: &DefinitionLocation<TFieldAssociatedData, TClientFieldType>,
) -> Option<&TFieldAssociatedData> {
    match definition_location {
        DefinitionLocation::Server(server_field) => Some(server_field),
        DefinitionLocation::Client(_) => None,
    }
}

pub fn as_client_type<TFieldAssociatedData, TClientFieldType>(
    definition_location: &DefinitionLocation<TFieldAssociatedData, TClientFieldType>,
) -> Option<&TClientFieldType> {
    match definition_location {
        DefinitionLocation::Server(_) => None,
        DefinitionLocation::Client(client_field) => Some(client_field),
    }
}
