use std::{fmt::Debug, marker::PhantomData};

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, ClientSelectableName, DescriptionValue,
    ObjectTypeAndFieldName, WithSpan,
};
use impl_base_types_macro::impl_for_selection_type;
use isograph_lang_types::{
    impl_with_id, ClientObjectSelectableId, ClientScalarSelectableId, SelectionType,
    ServerEntityId, ServerObjectEntityId, TypeAnnotation, VariableDefinition,
};

use crate::{
    ClientFieldVariant, NetworkProtocol, RefetchStrategy, ScalarSelectableId,
    UserWrittenClientPointerInfo, ValidatedObjectSelectionAssociatedData, ValidatedSelection,
};

pub type ClientSelectableId = SelectionType<ClientScalarSelectableId, ClientObjectSelectableId>;

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
    pub refetch_strategy:
        Option<RefetchStrategy<ScalarSelectableId, ValidatedObjectSelectionAssociatedData>>,

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
    pub to: TypeAnnotation<ServerObjectEntityId>,

    pub reader_selection_set: Vec<WithSpan<ValidatedSelection>>,

    pub refetch_strategy:
        RefetchStrategy<ScalarSelectableId, ValidatedObjectSelectionAssociatedData>,

    pub variable_definitions: Vec<WithSpan<VariableDefinition<ServerEntityId>>>,

    // Why is this not calculated when needed?
    pub type_and_field: ObjectTypeAndFieldName,

    pub parent_object_entity_id: ServerObjectEntityId,

    pub output_format: PhantomData<TNetworkProtocol>,
    pub info: UserWrittenClientPointerInfo,
}

impl_with_id!(ClientObjectSelectable<TNetworkProtocol: NetworkProtocol>, ClientObjectSelectableId);

#[impl_for_selection_type]
pub trait ClientScalarOrObjectSelectable {
    fn description(&self) -> Option<DescriptionValue>;
    fn name(&self) -> ClientSelectableName;
    fn type_and_field(&self) -> ObjectTypeAndFieldName;
    fn parent_object_entity_id(&self) -> ServerObjectEntityId;
    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>];
    fn refetch_strategy(
        &self,
    ) -> Option<&RefetchStrategy<ScalarSelectableId, ValidatedObjectSelectionAssociatedData>>;
    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>];

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityId>>];

    fn client_type(&self) -> &'static str;
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for &ClientScalarSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.into()
    }

    fn type_and_field(&self) -> ObjectTypeAndFieldName {
        self.type_and_field
    }

    fn parent_object_entity_id(&self) -> ServerObjectEntityId {
        self.parent_object_entity_id
    }

    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>] {
        &self.reader_selection_set
    }

    fn refetch_strategy(
        &self,
    ) -> Option<&RefetchStrategy<ScalarSelectableId, ValidatedObjectSelectionAssociatedData>> {
        self.refetch_strategy.as_ref()
    }

    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>] {
        match self.variant {
            ClientFieldVariant::ImperativelyLoadedField(_) => self
                .refetch_strategy
                .as_ref()
                .map(|strategy| strategy.refetch_selection_set())
                .expect(
                    "Expected imperatively loaded field to have refetch selection set. \
                    This is indicative of a bug in Isograph.",
                ),
            _ => &self.reader_selection_set,
        }
    }

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityId>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "field"
    }
}

impl<TNetworkProtocol: NetworkProtocol> ClientScalarOrObjectSelectable
    for &ClientObjectSelectable<TNetworkProtocol>
{
    fn description(&self) -> Option<DescriptionValue> {
        self.description
    }

    fn name(&self) -> ClientSelectableName {
        self.name.into()
    }

    fn type_and_field(&self) -> ObjectTypeAndFieldName {
        self.type_and_field
    }

    fn parent_object_entity_id(&self) -> ServerObjectEntityId {
        self.parent_object_entity_id
    }

    fn reader_selection_set(&self) -> &[WithSpan<ValidatedSelection>] {
        &self.reader_selection_set
    }

    fn refetch_strategy(
        &self,
    ) -> Option<&RefetchStrategy<ScalarSelectableId, ValidatedObjectSelectionAssociatedData>> {
        Some(&self.refetch_strategy)
    }

    fn selection_set_for_parent_query(&self) -> &[WithSpan<ValidatedSelection>] {
        &self.reader_selection_set
    }

    fn variable_definitions(&self) -> &[WithSpan<VariableDefinition<ServerEntityId>>] {
        &self.variable_definitions
    }

    fn client_type(&self) -> &'static str {
        "pointer"
    }
}
