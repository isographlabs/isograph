use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use common_lang_types::{
    DescriptionValue, Diagnostic, DiagnosticResult, EntityName, EntityNameAndSelectableName,
    RelativePathToSourceFile, SelectableName, SelectionName, VariableName,
};
use isograph_parser::{
    IsoLiteralItem, SelectableDeclaration, Selection, SelectionSet, VariableDeclarationList,
};
use nonempty::NonEmpty;
use pico_macros::memo;
use prelude::Postfix;

use crate::merged_selection::{
    Condition, DeclarationContext, FieldSelectionMapError, MergedArgument, field_variables,
    merged_arguments, source_field_declaration, undeclared_argument,
};
use crate::normalization_ast::{
    IntegerOutOfRange, NormalizationArguments, normalization_arguments,
};
use crate::query::{EntrypointQueryError, QueryArtifact, query_for_entrypoint};
use crate::reader_ast::{
    ReaderLinkedSelection, ReaderRefinementSelection, ReaderResolverSelection,
    ReaderScalarSelection, ReaderSelection, ReaderSelectionSet,
};
use crate::schema::{LiteralContextOf, source_iso_literal_items};
use crate::wrapper::wrapper_of_type_annotation;
use crate::{
    ArtifactSource, ClientId, ClientPointerTarget, CompilationProfile, EntityKind, EntityOrigin,
    HostLanguage, IsographDb, NetworkProtocol, ScalarKind, SelectableKey, SelectableKind,
    SelectableShape, Wrapper, entity_named, selectable_named, selectables_for_entity,
};

/// The type of a param type's `data`: one field per selection, in selection order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactObjectType(pub Vec<ArtifactObjectField>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactObjectField {
    /// The selection's alias, or its name when it has none.
    pub key: SelectionName,
    /// The selected selectable's description.
    pub description: Option<DescriptionValue>,
    pub type_: Wrapper<ArtifactInnerType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactInnerType {
    Scalar(ScalarKind),
    Object(ArtifactObjectType),
    /// What a client field's resolver returns: its `output_type` artifact.
    ClientFieldOutput(EntityNameAndSelectableName),
}

/// A field's variable definitions, and every input object they name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParametersType {
    pub parameters: NonEmpty<ArtifactParameter>,
    /// Every input object a parameter's type names, at any depth.
    pub input_objects: BTreeMap<EntityName, Vec<ArtifactInputField>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactParameter {
    pub name: VariableName,
    pub type_: Wrapper<ArtifactInputType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactInputField {
    pub name: SelectableName,
    pub type_: Wrapper<ArtifactInputType>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactInputType {
    Scalar(ScalarKind),
    /// Named; `ParametersType.input_objects` has its fields.
    InputObject(EntityName),
}

/// The one `field` declaration of a client field or pointer, with its source file and its
/// literal's host-language context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDeclaration<TLiteralContext> {
    pub path: RelativePathToSourceFile,
    pub context: TLiteralContext,
    pub declaration: SelectableDeclaration,
}

/// The declaration of `field` in the source files `client` reads.
#[memo]
pub fn field_declaration<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field: EntityNameAndSelectableName,
) -> DiagnosticResult<FieldDeclaration<LiteralContextOf<TProfile>>> {
    let EntityNameAndSelectableName {
        parent_entity_name,
        selectable_name,
    } = field;
    let mut declarations =
        source_iso_literal_items(db, client).filter_map(|source_item| match source_item.item {
            IsoLiteralItem::Selectable(declaration) => (declaration.parent_type.item.0
                == parent_entity_name
                && declaration.name.item.0 == selectable_name)
                .then(|| FieldDeclaration {
                    path: source_item.path,
                    context: source_item.context.clone(),
                    declaration: declaration.clone(),
                }),
            IsoLiteralItem::Entity(_) | IsoLiteralItem::Entrypoint(_) => None,
        });
    let Some(declaration) = declarations.next() else {
        return Diagnostic::new(
            format!("Field `{parent_entity_name}.{selectable_name}` is not defined."),
            None,
        )
        .wrap_err();
    };
    if declarations.next().is_some() {
        return Diagnostic::new(
            format!("Field `{parent_entity_name}.{selectable_name}` is defined more than once."),
            None,
        )
        .wrap_err();
    }
    declaration.wrap_ok()
}

/// What a field's generated types and reader say, and which client fields its selection set
/// selects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldTypes {
    pub data: ArtifactObjectType,
    pub parameters: Option<ParametersType>,
    /// One node per field of `data`, with the same keys in the same order.
    pub reader_ast: ReaderSelectionSet,
    /// Every client field or pointer the selection set selects at any depth, in selection order.
    pub selected: Vec<EntityNameAndSelectableName>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GenerationError {
    #[error("{}", .0.0.message)]
    Lookup(Diagnostic),
    #[error("`{}.{}` has a value with no fields, so it cannot have a selection set.", .0.entity, .0.selectable)]
    SelectionSetOnScalar(SelectableKey),
    #[error("`{}.{}` is a client field, so it cannot have a selection set.", .0.entity, .0.selectable)]
    SelectionSetOnClientField(SelectableKey),
    #[error("`{}.{}` points to an entity with fields, so it needs a selection set.", .0.entity, .0.selectable)]
    MissingSelectionSet(SelectableKey),
    #[error("The declaration of `{}.{}` did not parse completely.", .0.parent_entity_name, .0.selectable_name)]
    IncompleteParse(EntityNameAndSelectableName),
    #[error("{0}")]
    SelectionMap(FieldSelectionMapError),
    #[error("{0}")]
    IntegerOutOfRange(IntegerOutOfRange),
    #[error("`{}.{}` has no entity to select into.", .0.entity, .0.selectable)]
    NoTargetEntity(SelectableKey),
    #[error("The refinement `{}.{}` takes no arguments.", .0.entity, .0.selectable)]
    ArgumentsOnRefinement(SelectableKey),
    #[error("The refinement `{}.{}` targets a scalar.", .0.entity, .0.selectable)]
    RefinementToScalar(SelectableKey),
}

/// What a selection set on one entity contributes.
struct ObjectSelections {
    type_: ArtifactObjectType,
    reader: ReaderSelectionSet,
    selected: Vec<EntityNameAndSelectableName>,
}

/// What one selection contributes.
struct SelectionOutput {
    read: SelectionRead,
    selected: Vec<EntityNameAndSelectableName>,
}

/// Whether v1 reads a selection.
enum SelectionRead {
    /// Its field of the data type and its reader node, which have the same key.
    Read(ReadSelection),
    /// A client pointer: neither the data type nor the reader AST has it.
    NotRead,
}

struct ReadSelection {
    field: ArtifactObjectField,
    node: ReaderSelection,
}

/// What a server selection's target entity is, as the reader reads it.
enum ReaderTarget {
    Scalar,
    Object(ObjectTarget),
}

struct ObjectTarget {
    entity: EntityName,
    reader: ReaderSelectionSet,
}

/// The field whose selection set is being walked, and what its arguments are checked against.
#[derive(Clone, Copy)]
struct ReaderContext<'a> {
    field: EntityNameAndSelectableName,
    declaration: DeclarationContext<'a>,
}

/// The types and reader AST of `field` as `client` reads the project.
#[memo]
pub fn field_types<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field: EntityNameAndSelectableName,
) -> Result<FieldTypes, GenerationError> {
    let declaration = field_declaration(db, client, field)
        .clone()
        .map_err(GenerationError::Lookup)?
        .declaration;
    let EntityNameAndSelectableName {
        parent_entity_name,
        selectable_name,
    } = field;
    let source = source_field_declaration(db, client, parent_entity_name, selectable_name)
        .map_err(GenerationError::SelectionMap)?;
    let variables = field_variables(db, client, parent_entity_name, selectable_name)
        .as_ref()
        .map_err(|error| GenerationError::SelectionMap(error.clone()))?;
    let context = ReaderContext {
        field,
        declaration: DeclarationContext {
            path: source.path,
            literal_start: source.literal_start,
            variables: variables.as_slice(),
        },
    };
    let ObjectSelections {
        type_: data,
        reader: reader_ast,
        selected,
    } = match declaration.selection_set.as_ref() {
        None => ObjectSelections {
            type_: ArtifactObjectType(Vec::new()),
            reader: ReaderSelectionSet(Vec::new()),
            selected: Vec::new(),
        },
        Some(selection_set) => object_type(
            db,
            client,
            context,
            parent_entity_name,
            selection_set.item.reference(),
        )?,
    };
    let parameters = match declaration.variable_definitions.as_ref() {
        None => None,
        Some(variable_definitions) => {
            parameters_type(db, client, field, variable_definitions.item.reference())?
        }
    };
    FieldTypes {
        data,
        parameters,
        reader_ast,
        selected,
    }
    .wrap_ok()
}

/// The data type, reader AST, and selected client fields of `selection_set` on `entity`.
fn object_type<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: ReaderContext<'_>,
    entity: EntityName,
    selection_set: &SelectionSet,
) -> Result<ObjectSelections, GenerationError> {
    selection_set
        .0
        .iter()
        .map(|slot| {
            let selection = slot
                .item
                .item
                .as_ref()
                .ok_or(GenerationError::IncompleteParse(context.field))?;
            selection_field(db, client, context, entity, selection.item.reference())
        })
        .try_fold(
            ObjectSelections {
                type_: ArtifactObjectType(Vec::new()),
                reader: ReaderSelectionSet(Vec::new()),
                selected: Vec::new(),
            },
            |mut object, output| {
                let SelectionOutput { read, selected } = output?;
                match read {
                    SelectionRead::Read(ReadSelection { field, node }) => {
                        object.type_.0.push(field);
                        object.reader.0.push(node);
                    }
                    SelectionRead::NotRead => {}
                }
                object.selected.extend(selected);
                object.wrap_ok()
            },
        )
}

/// What one selection on `entity` contributes.
fn selection_field<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: ReaderContext<'_>,
    entity: EntityName,
    selection: &Selection,
) -> Result<SelectionOutput, GenerationError> {
    let name = selection.name.item.0;
    let selectable = selectable_named(db, client, entity, name.unchecked_conversion())
        .clone()
        .map_err(GenerationError::Lookup)?;
    let key = SelectableKey {
        entity,
        selectable: selectable.name,
    };
    let selected_self = EntityNameAndSelectableName {
        parent_entity_name: entity,
        selectable_name: selectable.name,
    };
    let alias = selection
        .reader_alias
        .map_or(name, |reader_alias| reader_alias.item.0);
    let data_field = |type_| ArtifactObjectField {
        key: alias,
        description: selectable.description,
        type_,
    };
    let nested_selection_set = selection
        .selection_set
        .as_ref()
        .map(|selection_set| selection_set.item.reference());
    match selectable.kind.reference() {
        SelectableKind::ClientField => {
            if nested_selection_set.is_some() {
                return GenerationError::SelectionSetOnClientField(key).wrap_err();
            }
            SelectionOutput {
                read: SelectionRead::Read(ReadSelection {
                    field: data_field(Wrapper::Inner(ArtifactInnerType::ClientFieldOutput(
                        selected_self,
                    ))),
                    node: ReaderSelection::Resolver(ReaderResolverSelection {
                        alias,
                        field: selected_self,
                        arguments: resolver_arguments(
                            db,
                            client,
                            context,
                            selected_self,
                            selection,
                        )?,
                    }),
                }),
                selected: selected_self.wrap_vec(),
            }
            .wrap_ok()
        }
        SelectableKind::ClientPointer(ClientPointerTarget::Parsed(target)) => {
            let nested_selection_set =
                nested_selection_set.ok_or(GenerationError::MissingSelectionSet(key))?;
            let nested_selected = target
                .inner_values()
                .map(|inner_entity| {
                    object_type(
                        db,
                        client,
                        context,
                        inner_entity.dereference(),
                        nested_selection_set,
                    )
                    .map(|nested| nested.selected)
                })
                .collect::<Result<Vec<_>, _>>()?;
            SelectionOutput {
                read: SelectionRead::NotRead,
                selected: std::iter::once(selected_self)
                    .chain(nested_selected.into_iter().flatten())
                    .collect(),
            }
            .wrap_ok()
        }
        SelectableKind::ClientPointer(ClientPointerTarget::Unparsed) => {
            GenerationError::IncompleteParse(context.field).wrap_err()
        }
        SelectableKind::Server(server) => {
            let arguments = reader_arguments(context, selection)?;
            let mut selected = Vec::new();
            let mut reader_target = None;
            let type_ = server.target.try_map(&mut |inner_entity| {
                let inner = entity_named(db, client, inner_entity.dereference())
                    .clone()
                    .map_err(GenerationError::Lookup)?;
                match (
                    inner.kind::<TProfile::NetworkProtocol>(),
                    nested_selection_set,
                ) {
                    (EntityKind::Scalar(_), Some(_)) => {
                        GenerationError::SelectionSetOnScalar(key).wrap_err()
                    }
                    (EntityKind::Scalar(scalar), None) => {
                        reader_target.get_or_insert(ReaderTarget::Scalar);
                        ArtifactInnerType::Scalar(scalar).wrap_ok()
                    }
                    (EntityKind::Object, None) => {
                        GenerationError::MissingSelectionSet(key).wrap_err()
                    }
                    (EntityKind::Object, Some(nested_selection_set)) => {
                        let nested = object_type(
                            db,
                            client,
                            context,
                            inner_entity.dereference(),
                            nested_selection_set,
                        )?;
                        selected.extend(nested.selected);
                        reader_target.get_or_insert(ReaderTarget::Object(ObjectTarget {
                            entity: inner_entity.dereference(),
                            reader: nested.reader,
                        }));
                        ArtifactInnerType::Object(nested.type_).wrap_ok()
                    }
                }
            })?;
            let reader_target = reader_target.ok_or(GenerationError::NoTargetEntity(key))?;
            let node = match (
                TProfile::NetworkProtocol::selectable_shape(server.data.reference()),
                reader_target,
            ) {
                (SelectableShape::Direct, ReaderTarget::Scalar) => {
                    ReaderSelection::Scalar(ReaderScalarSelection {
                        alias,
                        selectable: selectable.name,
                        arguments,
                    })
                }
                (SelectableShape::Direct, ReaderTarget::Object(object)) => {
                    ReaderSelection::Linked(ReaderLinkedSelection {
                        alias,
                        selectable: selectable.name,
                        arguments,
                        selections: object.reader,
                    })
                }
                (SelectableShape::Refinement, ReaderTarget::Object(object)) => {
                    if !arguments.0.is_empty() {
                        return GenerationError::ArgumentsOnRefinement(key).wrap_err();
                    }
                    ReaderSelection::Refinement(ReaderRefinementSelection {
                        alias,
                        condition: Condition::SelectionEquals(object.entity),
                        selections: object.reader,
                    })
                }
                (SelectableShape::Refinement, ReaderTarget::Scalar) => {
                    return GenerationError::RefinementToScalar(key).wrap_err();
                }
            };
            SelectionOutput {
                read: SelectionRead::Read(ReadSelection {
                    field: data_field(type_),
                    node,
                }),
                selected,
            }
            .wrap_ok()
        }
    }
}

/// `selection`'s arguments as the reader AST holds them: the enclosing field's variables stay
/// variables.
fn reader_arguments(
    context: ReaderContext<'_>,
    selection: &Selection,
) -> Result<NormalizationArguments, GenerationError> {
    let arguments = merged_arguments(context.declaration, selection.arguments.as_ref())
        .map_err(|error| GenerationError::SelectionMap(FieldSelectionMapError::Located(error)))?;
    normalization_arguments(arguments.merged.reference())
        .map_err(GenerationError::IntegerOutOfRange)
}

/// The arguments of a `Resolver` node for `selection`, a selection of the client field
/// `selected`: the arguments it passes, then the default of each of `selected`'s variables
/// it does not pass, sorted by name.
fn resolver_arguments<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: ReaderContext<'_>,
    selected: EntityNameAndSelectableName,
    selection: &Selection,
) -> Result<NormalizationArguments, GenerationError> {
    let arguments = merged_arguments(context.declaration, selection.arguments.as_ref())
        .map_err(|error| GenerationError::SelectionMap(FieldSelectionMapError::Located(error)))?;
    let variables = field_variables(
        db,
        client,
        selected.parent_entity_name,
        selected.selectable_name,
    )
    .as_ref()
    .map_err(|error| GenerationError::SelectionMap(error.clone()))?;
    if let Some(unknown) = undeclared_argument(
        context.declaration,
        SelectableKey {
            entity: selected.parent_entity_name,
            selectable: selected.selectable_name,
        },
        arguments.reference(),
        variables.as_slice(),
    ) {
        return GenerationError::SelectionMap(FieldSelectionMapError::Located(unknown)).wrap_err();
    }
    let defaults = variables.iter().filter_map(|variable| {
        variable
            .default_value
            .clone()
            .map(|default_value| MergedArgument {
                name: variable.name.unchecked_conversion(),
                value: default_value,
            })
    });
    normalization_arguments(arguments.merged.with_absent(defaults).reference())
        .map_err(GenerationError::IntegerOutOfRange)
}

/// The parameters type of `variable_definitions`; `None` when it declares no variable.
fn parameters_type<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    field: EntityNameAndSelectableName,
    variable_definitions: &VariableDeclarationList,
) -> Result<Option<ParametersType>, GenerationError> {
    let parameters = variable_definitions
        .0
        .iter()
        .map(|slot| {
            let declaration = slot
                .item
                .item
                .as_ref()
                .ok_or(GenerationError::IncompleteParse(field))?;
            let wrapper = wrapper_of_type_annotation(declaration.item.type_.item.reference())
                .ok_or(GenerationError::IncompleteParse(field))?;
            ArtifactParameter {
                name: declaration.item.name.item.0.item.0,
                type_: wrapper
                    .try_map(&mut |entity| input_type(db, client, entity.dereference()))?,
            }
            .wrap_ok()
        })
        .collect::<Result<Vec<_>, GenerationError>>()?;
    let Some(parameters) = NonEmpty::from_vec(parameters) else {
        return None.wrap_ok();
    };
    let input_objects = input_objects(
        db,
        client,
        parameters
            .iter()
            .flat_map(|parameter| input_objects_named(parameter.type_.reference()))
            .collect(),
    )?;
    ParametersType {
        parameters,
        input_objects,
    }
    .wrap_some()
    .wrap_ok()
}

/// The input type of a parameter or input field whose type names `entity`.
fn input_type<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
) -> Result<ArtifactInputType, GenerationError> {
    let named = entity_named(db, client, entity)
        .clone()
        .map_err(GenerationError::Lookup)?;
    match named.kind::<TProfile::NetworkProtocol>() {
        EntityKind::Scalar(scalar) => ArtifactInputType::Scalar(scalar),
        EntityKind::Object => ArtifactInputType::InputObject(entity),
    }
    .wrap_ok()
}

/// Every input object `type_` names.
fn input_objects_named(type_: &Wrapper<ArtifactInputType>) -> impl Iterator<Item = EntityName> {
    type_.inner_values().filter_map(|inner| match inner {
        ArtifactInputType::Scalar(_) => None,
        ArtifactInputType::InputObject(name) => name.dereference().wrap_some(),
    })
}

/// The fields of every input object in `pending` and of every input object those fields name.
fn input_objects<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    mut pending: Vec<EntityName>,
) -> Result<BTreeMap<EntityName, Vec<ArtifactInputField>>, GenerationError> {
    let mut expanded = BTreeMap::new();
    while let Some(input_object) = pending.pop() {
        let Entry::Vacant(slot) = expanded.entry(input_object) else {
            continue;
        };
        let fields = selectables_for_entity(db, client, input_object)
            .clone()
            .map_err(GenerationError::Lookup)?
            .into_iter()
            .map(|selectable| match selectable.kind {
                SelectableKind::Server(server) => ArtifactInputField {
                    name: selectable.name,
                    type_: server
                        .target
                        .try_map(&mut |entity| input_type(db, client, entity.dereference()))?,
                }
                .wrap_ok(),
                SelectableKind::ClientField | SelectableKind::ClientPointer(_) => {
                    GenerationError::Lookup(Diagnostic::new(
                        format!(
                            "`{input_object}.{}` is a client field on an input object.",
                            selectable.name
                        ),
                        None,
                    ))
                    .wrap_err()
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        pending.extend(
            fields
                .iter()
                .flat_map(|field| input_objects_named(field.type_.reference())),
        );
        slot.insert(fields);
    }
    expanded.wrap_ok()
}

/// Everything a host language needs to print every artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectArtifacts<TLiteralContext> {
    /// Sorted by field.
    pub fields: Vec<FieldArtifacts<TLiteralContext>>,
    /// The `entity` literals, sorted.
    pub entities: Vec<EntityName>,
    /// The `entrypoint` literals with their queries, sorted by entrypoint.
    pub entrypoints: Vec<EntrypointArtifacts>,
    /// One per entity some field is declared on or some pointer points to, sorted by entity.
    pub link_types: Vec<LinkType>,
}

/// An `entrypoint` literal and its query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntrypointArtifacts {
    pub entrypoint: EntityNameAndSelectableName,
    pub query: QueryArtifact,
}

/// Why the artifacts of a project could not be built.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, derive_more::From)]
pub enum ProjectArtifactsError<TNotFetchable: std::fmt::Display> {
    #[error("{0}")]
    Generation(GenerationError),
    #[error("{0}")]
    #[from(skip)]
    EntrypointQuery(EntrypointQueryFailed<TNotFetchable>),
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "The query of `entrypoint {}.{}` could not be generated: {error}",
    .entrypoint.parent_entity_name,
    .entrypoint.selectable_name
)]
pub struct EntrypointQueryFailed<TNotFetchable: std::fmt::Display> {
    pub entrypoint: EntityNameAndSelectableName,
    pub error: EntrypointQueryError<TNotFetchable>,
}

pub type ProfileProjectArtifactsError<TProfile> = ProjectArtifactsError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldArtifacts<TLiteralContext> {
    pub field: EntityNameAndSelectableName,
    pub kind: FieldArtifactKind,
    pub resolver: FieldDeclarationSource<TLiteralContext>,
    pub data: ArtifactObjectType,
    pub parameters: Option<ParametersType>,
    pub output_type: Generation,
    /// The reader AST, when the field is reachable from an entrypoint.
    pub resolver_reader: Option<ReaderSelectionSet>,
    /// The entities a link to `field.parent_entity_name` can name: the entity itself when
    /// concrete, its concrete entities when abstract, sorted.
    pub concrete_entities: Vec<EntityName>,
}

/// Where a field's resolver is: its source file and its literal's host-language context.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDeclarationSource<TLiteralContext> {
    pub path: RelativePathToSourceFile,
    pub context: TLiteralContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldArtifactKind {
    ClientField,
    Pointer(Wrapper<EntityName>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Generation {
    Generated,
    NotGenerated,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkType {
    pub entity: EntityName,
    /// The entity itself when concrete; its concrete types when abstract.
    pub concrete_types: Vec<EntityName>,
}

/// Project facts the host language reads when printing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationOptions<TCustomScalarType> {
    /// `<output_directory>/__isograph`, relative to the config file's directory.
    pub artifact_folder: PathBuf,
    /// The profile's `custom_scalar_types`.
    pub custom_scalar_types: BTreeMap<EntityName, TCustomScalarType>,
}

pub type ProfileGenerationOptions<TProfile> = GenerationOptions<
    <<TProfile as CompilationProfile>::HostLanguage as HostLanguage>::CustomScalarType,
>;

/// The artifacts of the project whose iso literals are `sources`, as `client` reads it.
pub fn project_artifacts<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    sources: &BTreeSet<ArtifactSource>,
) -> Result<ProjectArtifacts<LiteralContextOf<TProfile>>, ProfileProjectArtifactsError<TProfile>> {
    let entrypoints: Vec<EntityNameAndSelectableName> = sources
        .iter()
        .filter_map(|source| match source {
            ArtifactSource::Entrypoint(entrypoint) => entrypoint.dereference().wrap_some(),
            ArtifactSource::Entity(_) | ArtifactSource::Field(_) => None,
        })
        .collect();
    let entities: Vec<EntityName> = sources
        .iter()
        .filter_map(|source| match source {
            ArtifactSource::Entity(entity) => entity.dereference().wrap_some(),
            ArtifactSource::Field(_) | ArtifactSource::Entrypoint(_) => None,
        })
        .collect();
    let declared: Vec<EntityNameAndSelectableName> = sources
        .iter()
        .filter_map(|source| match source {
            ArtifactSource::Field(field) => field.dereference().wrap_some(),
            ArtifactSource::Entity(_) | ArtifactSource::Entrypoint(_) => None,
        })
        .collect();
    let types_of = |field: EntityNameAndSelectableName| field_types(db, client, field).clone();

    let mut reachable = BTreeSet::new();
    let mut pending = entrypoints.clone();
    while let Some(field) = pending.pop() {
        if reachable.insert(field) {
            pending.extend(types_of(field)?.selected.iter().copied());
        }
    }
    let output_types: BTreeSet<EntityNameAndSelectableName> = declared
        .iter()
        .map(|field| types_of(field.dereference()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|types| types.selected)
        .chain(entrypoints.iter().copied())
        .collect();
    let generation = |set: &BTreeSet<EntityNameAndSelectableName>,
                      field: EntityNameAndSelectableName| {
        if set.contains(field.reference()) {
            Generation::Generated
        } else {
            Generation::NotGenerated
        }
    };

    let fields = declared
        .iter()
        .map(|field| {
            let field = field.dereference();
            let types = types_of(field)?;
            let declaration = field_declaration(db, client, field)
                .clone()
                .map_err(GenerationError::Lookup)?;
            let selectable =
                selectable_named(db, client, field.parent_entity_name, field.selectable_name)
                    .clone()
                    .map_err(GenerationError::Lookup)?;
            let kind = match selectable.kind {
                SelectableKind::ClientField => FieldArtifactKind::ClientField,
                SelectableKind::ClientPointer(ClientPointerTarget::Parsed(target)) => {
                    FieldArtifactKind::Pointer(target)
                }
                SelectableKind::ClientPointer(ClientPointerTarget::Unparsed) => {
                    return GenerationError::IncompleteParse(field).wrap_err();
                }
                SelectableKind::Server(_) => {
                    return GenerationError::Lookup(Diagnostic::new(
                        format!(
                            "`{}.{}` is declared in a source file and defined by the network protocol.",
                            field.parent_entity_name, field.selectable_name
                        ),
                        None,
                    ))
                    .wrap_err();
                }
            };
            FieldArtifacts {
                field,
                kind,
                resolver: FieldDeclarationSource {
                    path: declaration.path,
                    context: declaration.context,
                },
                data: types.data,
                parameters: types.parameters,
                output_type: generation(output_types.reference(), field),
                resolver_reader: reachable.contains(field.reference()).then_some(types.reader_ast),
                concrete_entities: concrete_types(db, client, field.parent_entity_name)?,
            }
            .wrap_ok()
        })
        .collect::<Result<Vec<_>, GenerationError>>()?;

    let link_types = fields
        .iter()
        .flat_map(|field| {
            let pointed_to: Vec<EntityName> = match field.kind.reference() {
                FieldArtifactKind::Pointer(target) => target.inner_values().copied().collect(),
                FieldArtifactKind::ClientField => Vec::new(),
            };
            std::iter::once(field.field.parent_entity_name).chain(pointed_to)
        })
        .collect::<BTreeSet<EntityName>>()
        .into_iter()
        .map(|entity| {
            LinkType {
                entity,
                concrete_types: concrete_types(db, client, entity)?,
            }
            .wrap_ok()
        })
        .collect::<Result<Vec<_>, GenerationError>>()?;

    let entrypoints = entrypoints
        .into_iter()
        .map(|entrypoint| {
            query_for_entrypoint(
                db,
                client,
                entrypoint.parent_entity_name,
                entrypoint.selectable_name,
            )
            .clone()
            .map(|query| EntrypointArtifacts { entrypoint, query })
            .map_err(|error| {
                ProjectArtifactsError::EntrypointQuery(EntrypointQueryFailed { entrypoint, error })
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    ProjectArtifacts {
        fields,
        entities,
        entrypoints,
        link_types,
    }
    .wrap_ok()
}

/// The object types a link to `entity` can name.
fn concrete_types<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
) -> Result<Vec<EntityName>, GenerationError> {
    let named = entity_named(db, client, entity)
        .clone()
        .map_err(GenerationError::Lookup)?;
    match named.kind::<TProfile::NetworkProtocol>() {
        EntityKind::Scalar(_) => GenerationError::Lookup(Diagnostic::new(
            format!("`{entity}` is a scalar; a link names a record of an object entity."),
            None,
        ))
        .wrap_err(),
        EntityKind::Object => match named.origin {
            EntityOrigin::Source => entity.wrap_vec().wrap_ok(),
            EntityOrigin::Protocol(data) => {
                TProfile::NetworkProtocol::concrete_types(db, client, entity, data.reference())
                    .wrap_ok()
            }
        },
    }
}
