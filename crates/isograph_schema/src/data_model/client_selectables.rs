use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, DescriptionValue,
    ObjectTypeAndFieldName, WithSpan,
};
use isograph_lang_types::{
    impl_with_id, ClientObjectSelectableId, ClientScalarSelectableId, SelectionType,
    ServerEntityId, ServerObjectEntityId, TypeAnnotation, VariableDefinition,
};

use crate::{
    ClientFieldVariant, NetworkProtocol, ObjectSelectableId, RefetchStrategy, ScalarSelectableId,
    UserWrittenClientPointerInfo, ValidatedSelection,
};

pub type ClientSelectableId = SelectionType<ClientScalarSelectableId, ClientObjectSelectableId>;

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

    pub variable_definitions: Vec<WithSpan<VariableDefinition<ServerEntityId>>>,

    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldName,

    pub parent_object_entity_id: ServerObjectEntityId,
    pub output_format: PhantomData<TNetworkProtocol>,
}

impl_with_id!(ClientScalarSelectable<TNetworkProtocol: NetworkProtocol>, ClientScalarSelectableId);

/// The struct formally known as a client pointer, and declared with the pointer keyword
/// in iso literals.
#[derive(Debug)]
pub struct ClientObjectSelectable<TNetworkProtocol: NetworkProtocol> {
    pub description: Option<DescriptionValue>,
    pub name: ClientObjectSelectableName,
    pub target_object_entity: TypeAnnotation<ServerObjectEntityId>,

    pub reader_selection_set: Vec<WithSpan<ValidatedSelection>>,

    pub refetch_strategy: RefetchStrategy<ScalarSelectableId, ObjectSelectableId>,

    pub variable_definitions: Vec<WithSpan<VariableDefinition<ServerEntityId>>>,

    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldName,

    pub parent_object_entity_id: ServerObjectEntityId,

    pub output_format: PhantomData<TNetworkProtocol>,
    pub info: UserWrittenClientPointerInfo,
}

impl_with_id!(ClientObjectSelectable<TNetworkProtocol: NetworkProtocol>, ClientObjectSelectableId);
