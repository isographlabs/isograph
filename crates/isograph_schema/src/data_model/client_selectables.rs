use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, DescriptionValue,
    ObjectTypeAndFieldName, SchemaServerObjectEntityName, WithSpan,
};
use isograph_lang_types::{
    ClientObjectSelectableId, SelectionType, ServerEntityName, TypeAnnotation, VariableDefinition,
};

use crate::{
    ClientFieldVariant, NetworkProtocol, ObjectSelectableId, RefetchStrategy, ScalarSelectableId,
    UserWrittenClientPointerInfo, ValidatedSelection,
};

// TODO rename
pub type ClientSelectableId = SelectionType<
    (SchemaServerObjectEntityName, ClientScalarSelectableName),
    (SchemaServerObjectEntityName, ClientObjectSelectableId),
>;

pub type ClientSelectable<'a, TNetworkProtocol> = SelectionType<
    &'a ClientScalarSelectable<TNetworkProtocol>,
    &'a ClientObjectSelectable<TNetworkProtocol>,
>;

/// The struct formally known as a client field, and declared with the field keyword
/// in iso literals.
#[derive(Debug)]
pub struct ClientScalarSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: ClientScalarSelectableName,
    pub reader_selection_set: Vec<WithSpan<ValidatedSelection>>,

    // None -> not refetchable
    // TODO - this is only used if variant === imperatively loaded field
    // consider moving it into that struct.
    pub refetch_strategy: Option<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>,

    // TODO we should probably model this differently
    pub variant: ClientFieldVariant,

    pub variable_definitions: Vec<WithSpan<VariableDefinition<ServerEntityName>>>,

    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldName,

    pub parent_object_entity_name: SchemaServerObjectEntityName,
    pub output_format: PhantomData<TNetworkProtocol>,
}

/// The struct formally known as a client pointer, and declared with the pointer keyword
/// in iso literals.
#[derive(Debug)]
pub struct ClientObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: ClientObjectSelectableName,
    pub target_object_entity_name: TypeAnnotation<SchemaServerObjectEntityName>,

    pub reader_selection_set: Vec<WithSpan<ValidatedSelection>>,

    pub refetch_strategy: RefetchStrategy<ScalarSelectableId, ObjectSelectableId>,

    pub variable_definitions: Vec<WithSpan<VariableDefinition<ServerEntityName>>>,

    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldName,

    pub parent_object_name: SchemaServerObjectEntityName,

    pub output_format: PhantomData<TNetworkProtocol>,
    pub info: UserWrittenClientPointerInfo,
}
