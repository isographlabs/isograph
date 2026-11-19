use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};

use common_lang_types::{
    ArgumentName, Diagnostic, EntityName, RelativePathToSourceFile, SelectableName,
    StringLiteralValue, ValueKeyName, VariableName,
};
use intern::Lookup;
use isograph_parser::{
    ArgumentList, Boolean, IsoLiteralItem, NonConstantValue, SelectableDeclaration, Selection,
    SelectionSet, TypeAnnotation, UnionVariant,
};
use pico_macros::memo;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::validation::SelectableKey;
use crate::{
    ClientId, CompilationProfile, DefinitionLocation, EntityOrigin, EntityShape, IdentitySelection,
    IsographDb, NetworkProtocol, SelectableKind, SelectableShape, ServerSelectable, entity_named,
    parsed_iso_literals_in_file, selectable_named,
};

/// The entity a selection's selectable points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedTarget<TEntityData> {
    pub name: EntityName,
    pub data: TEntityData,
}

/// The server selections of one selection set after inlining `field` declarations, keyed by
/// what identifies them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedSelectionMap(BTreeMap<MergeKey, MergedSelection>);

/// What identifies a merged selection within its map: two selections with the same key are
/// one selection in the query.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MergeKey {
    Selectable(SelectionKey),
    Conditional(Condition),
}

/// One selectable selected with one set of argument values.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SelectionKey {
    pub name: SelectableName,
    pub arguments: MergedArguments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergedSelection {
    /// A selection of a protocol selectable.
    Selectable(MergedSelectableSelection),
    /// The selections of a refinement.
    Conditional(MergedConditional),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedSelectableSelection {
    pub key: SelectionKey,
    pub target: MergedSelectionTarget,
}

/// What a selectable selection's target entity is, which decides whether it has selections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergedSelectionTarget {
    Leaf,
    Composite(MergedCompositeTarget),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedCompositeTarget {
    /// The identity the protocol adds to a selection set on the target entity.
    pub identity: IdentitySelection,
    pub selections: MergedSelectionMap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergedConditional {
    pub condition: Condition,
    /// The identity the protocol adds to a selection set on the condition's entity, which
    /// the selections select on.
    pub identity: IdentitySelection,
    pub selections: MergedSelectionMap,
}

/// When a conditional's selections apply to the response object.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Condition {
    /// A refinement `asT`: the response object's discriminator selection equals `T`'s name.
    SelectionEquals(EntityName),
}

/// Arguments sorted by name, no name twice.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergedArguments(Vec<MergedArgument>);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergedArgument {
    pub name: ArgumentName,
    pub value: MergedValue,
}

/// An argument value with every variable of an inlined field replaced by the value its
/// selection passed. A remaining `Variable` names a variable of the field whose map this is.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MergedValue {
    Variable(VariableName),
    String(StringLiteralValue),
    Integer(i64),
    Boolean(Boolean),
    Null,
    Object(MergedObject),
}

/// Object entries sorted by name, no name twice.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergedObject(Vec<MergedObjectEntry>);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MergedObjectEntry {
    pub name: ValueKeyName,
    pub value: MergedValue,
}

/// The name a selection's value is stored under in the response and in the runtime store.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, derive_more::Display)]
pub struct NormalizationAlias(pub String);

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MergeError {
    #[error("{0}")]
    AliasCollision(AliasCollision),
    #[error("Two selections with one key have different targets.")]
    SelectionMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{}` and `{}` have the same normalization alias.", .first.name, .second.name)]
pub struct AliasCollision {
    pub first: SelectionKey,
    pub second: SelectionKey,
}

/// The merged selection map of a `field` declaration's selection set, and the variables it
/// declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSelectionMap {
    pub variables: Vec<FieldVariable>,
    pub selections: MergedSelectionMap,
}

/// `$name: Type = default` on a `field` declaration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldVariable {
    pub name: VariableName,
    pub variable_type: VariableType,
    /// A default contains no variables.
    pub default_value: Option<MergedValue>,
}

/// A variable's type: GraphQL's type grammar, which iso type annotations also write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariableType {
    NonNull(NonNullVariableType),
    Nullable(NonNullVariableType),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NonNullVariableType {
    Named(EntityName),
    List(Box<VariableType>),
}

/// Why a field's merged selection map could not be built: the error, and where it is.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{kind}")]
pub struct SelectionMapError {
    /// `None` when the error is the lookup of the field itself, which has no one definition.
    pub location: Option<DefinitionLocation>,
    pub kind: SelectionMapErrorKind,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SelectionMapErrorKind {
    #[error("{}", .0.0.message)]
    Lookup(Diagnostic),
    #[error("`{0}` comes from the schema and has no selection set to inline.")]
    NotASourceField(SelectableKey),
    #[error("The declaration of `{0}` is not where its definition says.")]
    DefinitionMoved(SelectableKey),
    #[error("This selection did not parse.")]
    UnparsedSelection,
    #[error("This argument did not parse.")]
    UnparsedArgument,
    #[error("This variable declaration did not parse.")]
    UnparsedVariable,
    #[error("This variable's type did not parse.")]
    UnparsedVariableType,
    #[error("A variable's type is a named type, a list, or one of those or `null`.")]
    UnsupportedVariableType,
    #[error("Variable `${0}` is not declared by this field.")]
    UndefinedVariable(VariableName),
    #[error("A default value cannot use the variable `${0}`.")]
    VariableInDefault(VariableName),
    #[error("Argument `{0}` is passed more than once.")]
    DuplicateArgument(ArgumentName),
    #[error("Key `{0}` appears more than once in this object.")]
    DuplicateObjectKey(ValueKeyName),
    #[error("List arguments are not supported in queries yet.")]
    ListArgument,
    #[error("`{}` declares no variable `${}`.", .0.field, .0.argument)]
    UnknownFieldArgument(UnknownFieldArgument),
    #[error("`{0}` has no entity to select into.")]
    NoTargetEntity(SelectableKey),
    #[error("`{0}` is declared in source files and cannot be the type of a schema field.")]
    TargetIsSourceEntity(EntityName),
    #[error("`{0}` returns a leaf and takes no selection set.")]
    SelectionSetOnLeaf(SelectableKey),
    #[error("`{0}` returns an entity with selectables and needs a selection set.")]
    MissingSelectionSet(SelectableKey),
    #[error("`{0}` is an input type, which cannot be selected.")]
    InputTarget(EntityName),
    #[error("The refinement `{0}` takes no arguments.")]
    ArgumentsOnRefinement(SelectableKey),
    #[error("A refinement cannot target the leaf `{0}`.")]
    RefinementToNonComposite(EntityName),
    #[error("These fields inline each other: {}.", .0.display())]
    InlineCycle(InlineCycle),
    #[error("{0}")]
    Merge(MergeError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownFieldArgument {
    pub field: SelectableKey,
    pub argument: ArgumentName,
}

/// A path of inlined fields from `start` that returns to a field on the path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineCycle(pub Vec<SelectableKey>);

impl InlineCycle {
    /// The keys joined with ` -> `.
    pub fn display(&self) -> String {
        self.0
            .iter()
            .map(SelectableKey::to_string)
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

impl MergedArguments {
    /// `arguments` sorted by name; `Err` is a name that appears twice.
    pub fn new(mut arguments: Vec<MergedArgument>) -> Result<Self, ArgumentName> {
        arguments.sort_by_key(|argument| argument.name);
        match arguments
            .windows(2)
            .find(|pair| pair[0].name == pair[1].name)
        {
            Some(pair) => pair[0].name.wrap_err(),
            None => MergedArguments(arguments).wrap_ok(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &MergedArgument> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl MergedObject {
    /// `entries` sorted by name; `Err` is a name that appears twice.
    pub fn new(mut entries: Vec<MergedObjectEntry>) -> Result<Self, ValueKeyName> {
        entries.sort_by_key(|entry| entry.name);
        match entries.windows(2).find(|pair| pair[0].name == pair[1].name) {
            Some(pair) => pair[0].name.wrap_err(),
            None => MergedObject(entries).wrap_ok(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &MergedObjectEntry> {
        self.0.iter()
    }
}

impl SelectionKey {
    /// The selectable name, then one chunk per argument: the name alone when the key has no
    /// arguments.
    pub fn normalization_alias(&self) -> NormalizationAlias {
        NormalizationAlias(
            std::iter::once(self.name.to_string())
                .chain(self.arguments.iter().map(|argument| {
                    format!("{}___{}", argument.name, argument.value.alias_chunk())
                }))
                .collect::<Vec<_>>()
                .join("____"),
        )
    }
}

impl MergedValue {
    /// The part of a normalization alias that names this value.
    fn alias_chunk(&self) -> String {
        match self {
            MergedValue::Variable(name) => format!("v_{name}"),
            MergedValue::Integer(integer) => format!("l_{integer}"),
            MergedValue::Boolean(Boolean::True) => "l_true".to_owned(),
            MergedValue::Boolean(Boolean::False) => "l_false".to_owned(),
            MergedValue::Null => "l_null".to_owned(),
            MergedValue::String(string) => format!(
                "s_{}",
                string
                    .lookup()
                    .chars()
                    .map(|character| match character {
                        'A'..='Z' | 'a'..='z' | '0'..='9' | '_' => character,
                        _ => '_',
                    })
                    .collect::<String>()
            ),
            MergedValue::Object(object) => format!(
                "o_{}_c",
                object
                    .iter()
                    .map(|entry| format!("{}__{}", entry.name, entry.value.alias_chunk()))
                    .collect::<Vec<_>>()
                    .join("_")
            ),
        }
    }

    /// Every variable name in this value, at any depth.
    fn variables(&self) -> BTreeSet<VariableName> {
        match self {
            MergedValue::Variable(name) => name.dereference().wrap_vec().into_iter().collect(),
            MergedValue::Object(object) => object
                .iter()
                .flat_map(|entry| entry.value.variables())
                .collect(),
            MergedValue::String(_)
            | MergedValue::Integer(_)
            | MergedValue::Boolean(_)
            | MergedValue::Null => BTreeSet::new(),
        }
    }
}

impl MergedSelection {
    /// `Selectable(s)` is `MergeKey::Selectable(s.key)`; `Conditional(c)` is
    /// `MergeKey::Conditional(c.condition)`.
    pub fn key(&self) -> MergeKey {
        match self {
            MergedSelection::Selectable(selection) => MergeKey::Selectable(selection.key.clone()),
            MergedSelection::Conditional(conditional) => {
                MergeKey::Conditional(conditional.condition.clone())
            }
        }
    }

    /// `self` and `other`, which have one key, as one selection.
    fn merge(self, other: Self) -> Result<Self, MergeError> {
        match (self, other) {
            (MergedSelection::Selectable(first), MergedSelection::Selectable(second)) => {
                MergedSelection::Selectable(MergedSelectableSelection {
                    key: first.key,
                    target: first.target.merge(second.target)?,
                })
                .wrap_ok()
            }
            (MergedSelection::Conditional(first), MergedSelection::Conditional(second)) => {
                if first.identity != second.identity {
                    return MergeError::SelectionMismatch.wrap_err();
                }
                MergedSelection::Conditional(MergedConditional {
                    condition: first.condition,
                    identity: first.identity,
                    selections: first.selections.merge(second.selections)?,
                })
                .wrap_ok()
            }
            (MergedSelection::Selectable(_), MergedSelection::Conditional(_))
            | (MergedSelection::Conditional(_), MergedSelection::Selectable(_)) => {
                MergeError::SelectionMismatch.wrap_err()
            }
        }
    }

    /// Every variable name in the arguments of this selection, at any depth.
    fn used_variables(&self) -> BTreeSet<VariableName> {
        match self {
            MergedSelection::Selectable(selection) => selection
                .key
                .arguments
                .iter()
                .flat_map(|argument| argument.value.variables())
                .chain(match selection.target.reference() {
                    MergedSelectionTarget::Leaf => BTreeSet::new(),
                    MergedSelectionTarget::Composite(target) => target.selections.used_variables(),
                })
                .collect(),
            MergedSelection::Conditional(conditional) => conditional.selections.used_variables(),
        }
    }
}

impl MergedSelectionTarget {
    /// `self` and `other`, the targets of one key, as one target.
    fn merge(self, other: Self) -> Result<Self, MergeError> {
        match (self, other) {
            (MergedSelectionTarget::Leaf, MergedSelectionTarget::Leaf) => {
                MergedSelectionTarget::Leaf.wrap_ok()
            }
            (MergedSelectionTarget::Composite(first), MergedSelectionTarget::Composite(second)) => {
                if first.identity != second.identity {
                    return MergeError::SelectionMismatch.wrap_err();
                }
                MergedSelectionTarget::Composite(MergedCompositeTarget {
                    identity: first.identity,
                    selections: first.selections.merge(second.selections)?,
                })
                .wrap_ok()
            }
            (MergedSelectionTarget::Leaf, MergedSelectionTarget::Composite(_))
            | (MergedSelectionTarget::Composite(_), MergedSelectionTarget::Leaf) => {
                MergeError::SelectionMismatch.wrap_err()
            }
        }
    }
}

impl MergedSelectionMap {
    pub fn empty() -> Self {
        MergedSelectionMap(BTreeMap::new())
    }

    /// The map holding only `selection`.
    pub fn single(selection: MergedSelection) -> Self {
        MergedSelectionMap(std::iter::once((selection.key(), selection)).collect())
    }

    /// `self` with every selection of `other` merged in.
    pub fn merge(self, other: Self) -> Result<Self, MergeError> {
        other
            .0
            .into_values()
            .try_fold(self, MergedSelectionMap::insert)?
            .without_alias_collision()
            .map_err(MergeError::AliasCollision)
    }

    /// The selections in key order.
    pub fn iter(&self) -> impl Iterator<Item = &MergedSelection> {
        self.0.values()
    }

    /// Every variable name in the arguments of this map, at any depth.
    pub fn used_variables(&self) -> BTreeSet<VariableName> {
        self.iter()
            .flat_map(MergedSelection::used_variables)
            .collect()
    }

    /// `self` with `selection` at its key, merged with the selection already there.
    fn insert(mut self, selection: MergedSelection) -> Result<Self, MergeError> {
        match self.0.entry(selection.key()) {
            Entry::Vacant(vacant) => {
                vacant.insert(selection);
            }
            Entry::Occupied(occupied) => {
                let (key, existing) = occupied.remove_entry();
                self.0.insert(key, existing.merge(selection)?);
            }
        }
        self.wrap_ok()
    }

    /// `Err` when two selectable keys of this map have the same normalization alias.
    fn without_alias_collision(self) -> Result<Self, AliasCollision> {
        self.0
            .keys()
            .filter_map(|key| match key {
                MergeKey::Selectable(key) => key.wrap_some(),
                MergeKey::Conditional(_) => None,
            })
            .try_fold(BTreeMap::new(), |mut by_alias, key| {
                match by_alias.entry(key.normalization_alias()) {
                    Entry::Vacant(vacant) => {
                        vacant.insert(key);
                        by_alias.wrap_ok()
                    }
                    Entry::Occupied(occupied) => AliasCollision {
                        first: occupied.get().dereference().clone(),
                        second: key.clone(),
                    }
                    .wrap_err(),
                }
            })?;
        self.wrap_ok()
    }
}

/// What each variable of an inlined field is replaced by.
#[derive(Clone, Debug, PartialEq, Eq)]
enum VariableValue {
    Value(MergedValue),
    /// The selection passed no argument and the variable has no default.
    Unset,
}

/// `field_map` with each variable replaced by its value in `context`.
fn substitute(
    field_map: &MergedSelectionMap,
    context: &BTreeMap<VariableName, VariableValue>,
) -> Result<MergedSelectionMap, SelectionMapErrorKind> {
    field_map
        .iter()
        .try_fold(MergedSelectionMap::empty(), |map, selection| {
            map.insert(substitute_selection(selection, context)?)
                .map_err(SelectionMapErrorKind::Merge)
        })?
        .without_alias_collision()
        .map_err(|collision| SelectionMapErrorKind::Merge(MergeError::AliasCollision(collision)))
}

/// `selection` with each variable replaced by its value in `context`, at any depth.
fn substitute_selection(
    selection: &MergedSelection,
    context: &BTreeMap<VariableName, VariableValue>,
) -> Result<MergedSelection, SelectionMapErrorKind> {
    match selection {
        MergedSelection::Selectable(selection) => {
            MergedSelection::Selectable(MergedSelectableSelection {
                key: SelectionKey {
                    name: selection.key.name,
                    arguments: substitute_arguments(selection.key.arguments.reference(), context)?,
                },
                target: match selection.target.reference() {
                    MergedSelectionTarget::Leaf => MergedSelectionTarget::Leaf,
                    MergedSelectionTarget::Composite(target) => {
                        MergedSelectionTarget::Composite(MergedCompositeTarget {
                            identity: target.identity,
                            selections: substitute(target.selections.reference(), context)?,
                        })
                    }
                },
            })
        }
        MergedSelection::Conditional(conditional) => {
            MergedSelection::Conditional(MergedConditional {
                condition: conditional.condition.clone(),
                identity: conditional.identity,
                selections: substitute(conditional.selections.reference(), context)?,
            })
        }
    }
    .wrap_ok()
}

/// `arguments` with each variable replaced by its value in `context`; an argument whose value
/// is an unset variable is removed.
fn substitute_arguments(
    arguments: &MergedArguments,
    context: &BTreeMap<VariableName, VariableValue>,
) -> Result<MergedArguments, SelectionMapErrorKind> {
    arguments
        .iter()
        .map(|argument| {
            let value = match argument.value.reference() {
                MergedValue::Variable(name) => match variable_value(name.dereference(), context)? {
                    VariableValue::Value(value) => value.wrap_some(),
                    VariableValue::Unset => None,
                },
                value => substitute_value(value, context)?.wrap_some(),
            };
            value
                .map(|value| MergedArgument {
                    name: argument.name,
                    value,
                })
                .wrap_ok()
        })
        .filter_map(Result::transpose)
        .collect::<Result<Vec<_>, _>>()
        .map(MergedArguments)
}

/// `value` with each variable replaced by its value in `context`; an unset variable is `null`.
fn substitute_value(
    value: &MergedValue,
    context: &BTreeMap<VariableName, VariableValue>,
) -> Result<MergedValue, SelectionMapErrorKind> {
    match value {
        MergedValue::Variable(name) => match variable_value(name.dereference(), context)? {
            VariableValue::Value(value) => value.wrap_ok(),
            VariableValue::Unset => MergedValue::Null.wrap_ok(),
        },
        MergedValue::Object(object) => object
            .iter()
            .map(|entry| {
                MergedObjectEntry {
                    name: entry.name,
                    value: substitute_value(entry.value.reference(), context)?,
                }
                .wrap_ok()
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|entries| MergedValue::Object(MergedObject(entries))),
        MergedValue::String(_)
        | MergedValue::Integer(_)
        | MergedValue::Boolean(_)
        | MergedValue::Null => value.clone().wrap_ok(),
    }
}

/// The value of `name` in `context`. Every variable of a field's map is declared by the
/// field, and `context` holds every declared variable.
fn variable_value(
    name: VariableName,
    context: &BTreeMap<VariableName, VariableValue>,
) -> Result<VariableValue, SelectionMapErrorKind> {
    context
        .get(name.reference())
        .cloned()
        .ok_or(SelectionMapErrorKind::UndefinedVariable(name))
}

/// A `field` declaration in a source file, borrowed from its literal's parse.
pub struct SourceFieldDeclaration<'db> {
    pub path: RelativePathToSourceFile,
    /// The byte offset of the literal in its file; spans in `declaration` are relative to it.
    pub literal_start: u32,
    pub declaration: &'db SelectableDeclaration,
}

/// The declaration of `entity.selectable`, found from the selectable's `definition`.
pub fn source_field_declaration<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<SourceFieldDeclaration<'_>, SelectionMapError> {
    let key = SelectableKey { entity, selectable };
    let found = selectable_named(db, client, entity, selectable)
        .as_ref()
        .map_err(|diagnostic| SelectionMapError {
            location: None,
            kind: SelectionMapErrorKind::Lookup(diagnostic.clone()),
        })?;
    let definition = found.definition;
    let error = |kind| SelectionMapError {
        location: definition.wrap_some(),
        kind,
    };
    match found.kind {
        SelectableKind::Server(_) => {
            return error(SelectionMapErrorKind::NotASourceField(key)).wrap_err();
        }
        SelectableKind::ClientField | SelectableKind::ClientPointer(_) => {}
    }
    parsed_iso_literals_in_file(db, client, definition.path)
        .iter()
        .flatten()
        .filter(|(extraction, _)| extraction.span().contains(definition.span))
        .find_map(|(extraction, parsed)| {
            let literal_start = u32::try_from(extraction.iso_literal_start_index.0).ok()?;
            match parsed.item.as_ref()?.item.item.as_ref()?.item.reference() {
                IsoLiteralItem::Selectable(declaration)
                    if declaration.parent_type.item.0 == entity
                        && declaration.name.item.0 == selectable
                        && declaration.name.location.with_offset(literal_start)
                            == definition.span =>
                {
                    SourceFieldDeclaration {
                        path: definition.path,
                        literal_start,
                        declaration,
                    }
                    .wrap_some()
                }
                IsoLiteralItem::Selectable(_)
                | IsoLiteralItem::Entity(_)
                | IsoLiteralItem::Entrypoint(_) => None,
            }
        })
        .ok_or_else(|| error(SelectionMapErrorKind::DefinitionMoved(key)))
}

/// What a selection set's errors are located against: the declaration it is in.
#[derive(Clone, Copy)]
struct DeclarationContext<'a> {
    path: RelativePathToSourceFile,
    literal_start: u32,
    variables: &'a [FieldVariable],
}

impl DeclarationContext<'_> {
    /// `error`, whose span is relative to the literal, located in its file.
    fn located(&self, error: WithSpan<SelectionMapErrorKind>) -> SelectionMapError {
        SelectionMapError {
            location: DefinitionLocation {
                path: self.path,
                span: error.location.with_offset(self.literal_start),
            }
            .wrap_some(),
            kind: error.item,
        }
    }
}

/// The merged selection map of the `field` declaration `entity.selectable` as `client`
/// reads the source and schema.
#[memo]
pub fn field_selection_map<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<FieldSelectionMap, SelectionMapError> {
    let field = source_field_declaration(db, client, entity, selectable)?;
    let located = |error: WithSpan<SelectionMapErrorKind>| {
        DeclarationContext {
            path: field.path,
            literal_start: field.literal_start,
            variables: &[],
        }
        .located(error)
    };
    if let Some(cycle) = inline_cycle(db, client, SelectableKey { entity, selectable }) {
        return located(
            SelectionMapErrorKind::InlineCycle(cycle).with_span(field.declaration.name.location),
        )
        .wrap_err();
    }
    let variables = field_variables(field.declaration).map_err(located)?;
    let context = DeclarationContext {
        path: field.path,
        literal_start: field.literal_start,
        variables: variables.as_slice(),
    };
    let selections = match field.declaration.selection_set.as_ref() {
        Some(selection_set) => {
            merge_selection_set(db, client, context, entity, selection_set.item.reference())?
        }
        None => MergedSelectionMap::empty(),
    };
    FieldSelectionMap {
        variables,
        selections,
    }
    .wrap_ok()
}

/// The variables `declaration` declares, in source order.
fn field_variables(
    declaration: &SelectableDeclaration,
) -> Result<Vec<FieldVariable>, WithSpan<SelectionMapErrorKind>> {
    declaration
        .variable_definitions
        .iter()
        .flat_map(|list| list.item.0.iter())
        .map(|slot| {
            let variable = slot
                .item
                .item
                .as_ref()
                .ok_or(SelectionMapErrorKind::UnparsedVariable.with_span(slot.location))?;
            FieldVariable {
                name: variable.item.name.item.0.item.0,
                variable_type: variable_type(variable.item.type_.reference())?,
                default_value: variable
                    .item
                    .default_value
                    .as_ref()
                    .map(|default| merged_value(default, VariableScope::Default))
                    .transpose()?,
            }
            .wrap_ok()
        })
        .collect()
}

/// `annotation` as a variable type: `T` is non-null, `T | null` is nullable.
fn variable_type(
    annotation: &WithSpan<TypeAnnotation>,
) -> Result<VariableType, WithSpan<SelectionMapErrorKind>> {
    let unparsed = || SelectionMapErrorKind::UnparsedVariableType.with_span(annotation.location);
    let unsupported =
        || SelectionMapErrorKind::UnsupportedVariableType.with_span(annotation.location);
    match annotation.item.reference() {
        TypeAnnotation::Named(named) => {
            VariableType::NonNull(NonNullVariableType::Named(named.name.item.0)).wrap_ok()
        }
        TypeAnnotation::List(list) => VariableType::NonNull(NonNullVariableType::List(
            variable_type(list.inner.as_ref().ok_or_else(unparsed)?)?.boxed(),
        ))
        .wrap_ok(),
        TypeAnnotation::Union(union) => {
            let (nulls, non_nulls): (Vec<_>, Vec<_>) = union
                .0
                .iter()
                .partition(|variant| matches!(variant.item, UnionVariant::Null(_)));
            match (nulls.as_slice(), non_nulls.as_slice()) {
                ([_], [variant]) => VariableType::Nullable(match variant.item.reference() {
                    UnionVariant::Named(named) => NonNullVariableType::Named(named.name.item.0),
                    UnionVariant::List(list) => NonNullVariableType::List(
                        variable_type(list.inner.as_ref().ok_or_else(unparsed)?)?.boxed(),
                    ),
                    UnionVariant::Null(_) => return unsupported().wrap_err(),
                })
                .wrap_ok(),
                _ => unsupported().wrap_err(),
            }
        }
    }
}

/// The variables a value may use.
#[derive(Clone, Copy)]
enum VariableScope<'a> {
    /// A selection's argument: the variables of the declaration it is in.
    Declaration(&'a [FieldVariable]),
    /// A variable's default value, which uses none.
    Default,
}

/// `value` as a merged value.
fn merged_value(
    value: &WithSpan<NonConstantValue>,
    scope: VariableScope<'_>,
) -> Result<MergedValue, WithSpan<SelectionMapErrorKind>> {
    match value.item.reference() {
        NonConstantValue::Variable(variable) => {
            let name = variable.0.item.0.item.0;
            let undeclared = match scope {
                VariableScope::Default => {
                    SelectionMapErrorKind::VariableInDefault(name).wrap_some()
                }
                VariableScope::Declaration(variables) => {
                    (!variables.iter().any(|variable| variable.name == name))
                        .then_some(SelectionMapErrorKind::UndefinedVariable(name))
                }
            };
            match undeclared {
                Some(kind) => kind.with_span(value.location).wrap_err(),
                None => MergedValue::Variable(name).wrap_ok(),
            }
        }
        NonConstantValue::String(string) => MergedValue::String(string.0).wrap_ok(),
        NonConstantValue::Integer(integer) => MergedValue::Integer(integer.0).wrap_ok(),
        NonConstantValue::Boolean(boolean) => MergedValue::Boolean(boolean.0).wrap_ok(),
        NonConstantValue::Null(_) => MergedValue::Null.wrap_ok(),
        NonConstantValue::Object(object) => object
            .0
            .iter()
            .map(|slot| {
                let entry = slot
                    .item
                    .item
                    .as_ref()
                    .ok_or(SelectionMapErrorKind::UnparsedArgument.with_span(slot.location))?;
                MergedObjectEntry {
                    name: entry.item.name.item.0,
                    value: merged_value(entry.item.value.reference(), scope)?,
                }
                .wrap_ok()
            })
            .collect::<Result<Vec<_>, _>>()
            .and_then(|entries| {
                MergedObject::new(entries).map_err(|name| {
                    SelectionMapErrorKind::DuplicateObjectKey(name).with_span(value.location)
                })
            })
            .map(MergedValue::Object),
        NonConstantValue::List(_) => SelectionMapErrorKind::ListArgument
            .with_span(value.location)
            .wrap_err(),
    }
}

/// A selection's arguments, each with its span, and as merged arguments.
struct SelectionArguments {
    located: Vec<WithSpan<MergedArgument>>,
    merged: MergedArguments,
}

fn selection_arguments(
    arguments: Option<&WithSpan<ArgumentList>>,
    scope: VariableScope<'_>,
) -> Result<SelectionArguments, WithSpan<SelectionMapErrorKind>> {
    let Some(arguments) = arguments else {
        return SelectionArguments {
            located: Vec::new(),
            merged: MergedArguments::default(),
        }
        .wrap_ok();
    };
    let located = arguments
        .item
        .0
        .iter()
        .map(|slot| {
            let argument = slot
                .item
                .item
                .as_ref()
                .ok_or(SelectionMapErrorKind::UnparsedArgument.with_span(slot.location))?;
            MergedArgument {
                name: argument.item.name.item.0,
                value: merged_value(argument.item.value.reference(), scope)?,
            }
            .with_span(argument.location)
            .wrap_ok()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let merged = MergedArguments::new(
        located
            .iter()
            .map(|argument| argument.item.clone())
            .collect(),
    )
    .map_err(|name| SelectionMapErrorKind::DuplicateArgument(name).with_span(arguments.location))?;
    SelectionArguments { located, merged }.wrap_ok()
}

/// The merged selection map of `selection_set`, a selection set on `parent`.
fn merge_selection_set<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: DeclarationContext<'_>,
    parent: EntityName,
    selection_set: &SelectionSet,
) -> Result<MergedSelectionMap, SelectionMapError> {
    selection_set
        .0
        .iter()
        .try_fold(MergedSelectionMap::empty(), |map, slot| {
            let selection = slot.item.item.as_ref().ok_or_else(|| {
                context.located(SelectionMapErrorKind::UnparsedSelection.with_span(slot.location))
            })?;
            let contribution =
                selection_contribution(db, client, context, parent, selection.item.reference())?;
            map.merge(contribution).map_err(|error| {
                context.located(
                    SelectionMapErrorKind::Merge(error).with_span(selection.item.name.location),
                )
            })
        })
}

/// What one selection on `parent` adds to its selection set's map.
fn selection_contribution<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: DeclarationContext<'_>,
    parent: EntityName,
    selection: &Selection,
) -> Result<MergedSelectionMap, SelectionMapError> {
    let name: SelectableName = selection.name.item.0.unchecked_conversion();
    let selectable = selectable_named(db, client, parent, name)
        .as_ref()
        .map_err(|diagnostic| {
            context.located(
                SelectionMapErrorKind::Lookup(diagnostic.clone())
                    .with_span(selection.name.location),
            )
        })?;
    let arguments = selection_arguments(
        selection.arguments.as_ref(),
        VariableScope::Declaration(context.variables),
    )
    .map_err(|error| context.located(error))?;
    let key = SelectableKey {
        entity: parent,
        selectable: name,
    };
    match selectable.kind.reference() {
        SelectableKind::Server(server) => server_contribution(
            db,
            client,
            context,
            key,
            selection,
            arguments.merged,
            server,
        ),
        SelectableKind::ClientField => {
            client_field_contribution(db, client, context, key, selection, arguments)
        }
        SelectableKind::ClientPointer(_) => MergedSelectionMap::empty().wrap_ok(),
    }
}

/// What `selection`, a selection of the protocol selectable `key` passing `arguments`, adds
/// to its selection set's map.
fn server_contribution<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: DeclarationContext<'_>,
    key: SelectableKey,
    selection: &Selection,
    arguments: MergedArguments,
    server: &ServerSelectable<<TProfile::NetworkProtocol as NetworkProtocol>::SelectableData>,
) -> Result<MergedSelectionMap, SelectionMapError> {
    let at_name =
        |kind: SelectionMapErrorKind| context.located(kind.with_span(selection.name.location));
    let target_name = server
        .target
        .inner_values()
        .next()
        .copied()
        .ok_or_else(|| at_name(SelectionMapErrorKind::NoTargetEntity(key)))?;
    let target = entity_named(db, client, target_name)
        .as_ref()
        .map_err(|diagnostic| at_name(SelectionMapErrorKind::Lookup(diagnostic.clone())))?;
    let data = match target.origin.reference() {
        EntityOrigin::Protocol(data) => data,
        EntityOrigin::Source => {
            return at_name(SelectionMapErrorKind::TargetIsSourceEntity(target_name)).wrap_err();
        }
    };
    let nested_map = || {
        let nested = selection
            .selection_set
            .as_ref()
            .ok_or_else(|| at_name(SelectionMapErrorKind::MissingSelectionSet(key)))?;
        merge_selection_set(db, client, context, target_name, nested.item.reference())
    };
    let selection_key = SelectionKey {
        name: key.selectable,
        arguments,
    };
    match (
        TProfile::NetworkProtocol::selectable_shape(server.data.reference()),
        TProfile::NetworkProtocol::entity_shape(data),
    ) {
        (SelectableShape::Direct | SelectableShape::Refinement, EntityShape::Input) => {
            at_name(SelectionMapErrorKind::InputTarget(target_name)).wrap_err()
        }
        (SelectableShape::Direct, EntityShape::Leaf) => match selection.selection_set {
            Some(_) => at_name(SelectionMapErrorKind::SelectionSetOnLeaf(key)).wrap_err(),
            None => {
                MergedSelectionMap::single(MergedSelection::Selectable(MergedSelectableSelection {
                    key: selection_key,
                    target: MergedSelectionTarget::Leaf,
                }))
                .wrap_ok()
            }
        },
        (SelectableShape::Direct, EntityShape::Composite(identity)) => {
            MergedSelectionMap::single(MergedSelection::Selectable(MergedSelectableSelection {
                key: selection_key,
                target: MergedSelectionTarget::Composite(MergedCompositeTarget {
                    identity,
                    selections: nested_map()?,
                }),
            }))
            .wrap_ok()
        }
        (SelectableShape::Refinement, EntityShape::Leaf) => {
            at_name(SelectionMapErrorKind::RefinementToNonComposite(target_name)).wrap_err()
        }
        (SelectableShape::Refinement, EntityShape::Composite(identity)) => {
            if !selection_key.arguments.is_empty() {
                return at_name(SelectionMapErrorKind::ArgumentsOnRefinement(key)).wrap_err();
            }
            MergedSelectionMap::single(MergedSelection::Conditional(MergedConditional {
                condition: Condition::SelectionEquals(target_name),
                identity,
                selections: nested_map()?,
            }))
            .wrap_ok()
        }
    }
}

/// What `selection`, a selection of the client field `key` passing `arguments`, adds to its
/// selection set's map: the field's map with the values it passed.
fn client_field_contribution<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    context: DeclarationContext<'_>,
    key: SelectableKey,
    selection: &Selection,
    arguments: SelectionArguments,
) -> Result<MergedSelectionMap, SelectionMapError> {
    let field = field_selection_map(db, client, key.entity, key.selectable)
        .as_ref()
        .map_err(Clone::clone)?;
    if let Some(unknown) = arguments.located.iter().find(|argument| {
        !field
            .variables
            .iter()
            .any(|variable| variable.name.lookup() == argument.item.name.lookup())
    }) {
        return context
            .located(
                SelectionMapErrorKind::UnknownFieldArgument(UnknownFieldArgument {
                    field: key,
                    argument: unknown.item.name,
                })
                .with_span(unknown.location),
            )
            .wrap_err();
    }
    let values = field
        .variables
        .iter()
        .map(|variable| {
            let passed = arguments
                .merged
                .iter()
                .find(|argument| argument.name.lookup() == variable.name.lookup())
                .map(|argument| argument.value.clone());
            let value = match passed.or_else(|| variable.default_value.clone()) {
                Some(value) => VariableValue::Value(value),
                None => VariableValue::Unset,
            };
            (variable.name, value)
        })
        .collect::<BTreeMap<_, _>>();
    substitute(field.selections.reference(), values.reference())
        .map_err(|kind| context.located(kind.with_span(selection.name.location)))
}

/// The `field` declarations whose selection sets `entity.selectable`'s selection set
/// inlines directly: client field selections at any depth under protocol selections. A
/// selection whose lookup fails is not listed, and neither is a client pointer.
#[memo]
pub fn inlined_fields<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> BTreeSet<SelectableKey> {
    let Ok(field) = source_field_declaration(db, client, entity, selectable) else {
        return BTreeSet::new();
    };
    field
        .declaration
        .selection_set
        .iter()
        .flat_map(|selection_set| {
            inlined_in_selection_set(db, client, entity, selection_set.item.reference())
        })
        .collect()
}

fn inlined_in_selection_set<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
    selection_set: &SelectionSet,
) -> BTreeSet<SelectableKey> {
    selection_set
        .0
        .iter()
        .filter_map(|slot| slot.item.item.as_ref())
        .flat_map(|selection| inlined_in_selection(db, client, parent, selection.item.reference()))
        .collect()
}

fn inlined_in_selection<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    parent: EntityName,
    selection: &Selection,
) -> BTreeSet<SelectableKey> {
    let name: SelectableName = selection.name.item.0.unchecked_conversion();
    let Ok(selectable) = selectable_named(db, client, parent, name) else {
        return BTreeSet::new();
    };
    match selectable.kind.reference() {
        SelectableKind::Server(server) => {
            match (
                server.target.inner_values().next(),
                selection.selection_set.as_ref(),
            ) {
                (Some(target), Some(nested)) => inlined_in_selection_set(
                    db,
                    client,
                    target.dereference(),
                    nested.item.reference(),
                ),
                (None, _) | (_, None) => BTreeSet::new(),
            }
        }
        SelectableKind::ClientField => SelectableKey {
            entity: parent,
            selectable: name,
        }
        .wrap_vec()
        .into_iter()
        .collect(),
        SelectableKind::ClientPointer(_) => BTreeSet::new(),
    }
}

/// The first cycle of inlined fields reachable from `start`, by depth-first search over
/// `inlined_fields`.
fn inline_cycle<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    start: SelectableKey,
) -> Option<InlineCycle> {
    let children = |key: SelectableKey| {
        inlined_fields(db, client, key.entity, key.selectable)
            .iter()
            .copied()
    };
    let mut path = vec![(start, children(start))];
    let mut finished = BTreeSet::new();
    loop {
        let (_, remaining) = path.last_mut()?;
        match remaining.next() {
            None => {
                if let Some((key, _)) = path.pop() {
                    finished.insert(key);
                }
            }
            Some(child) => {
                if let Some(position) = path.iter().position(|(key, _)| *key == child) {
                    return InlineCycle(path.iter().skip(position).map(|(key, _)| *key).collect())
                        .wrap_some();
                }
                if !finished.contains(&child) {
                    path.push((child, children(child)));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use common_lang_types::{ArgumentName, EntityName, SelectableName, ValueKeyName, VariableName};
    use intern::string_key::Intern;
    use isograph_parser::Boolean;
    use prelude::Postfix;

    use super::{
        AliasCollision, Condition, MergeError, MergeKey, MergedArgument, MergedArguments,
        MergedCompositeTarget, MergedConditional, MergedObject, MergedObjectEntry,
        MergedSelectableSelection, MergedSelection, MergedSelectionMap, MergedSelectionTarget,
        MergedValue, SelectionKey, SelectionMapErrorKind, VariableValue, substitute,
    };
    use crate::IdentitySelection;

    fn name(s: &str) -> SelectableName {
        s.intern().to()
    }

    fn entity(s: &str) -> EntityName {
        s.intern().to()
    }

    fn variable(s: &str) -> VariableName {
        s.intern().to()
    }

    fn argument_name(s: &str) -> ArgumentName {
        s.intern().to()
    }

    fn argument(s: &str, value: MergedValue) -> MergedArgument {
        MergedArgument {
            name: argument_name(s),
            value,
        }
    }

    fn arguments(list: Vec<MergedArgument>) -> MergedArguments {
        MergedArguments::new(list).expect("the test passes each argument once")
    }

    fn var(s: &str) -> MergedValue {
        MergedValue::Variable(variable(s))
    }

    fn string(s: &str) -> MergedValue {
        MergedValue::String(s.intern().to())
    }

    fn object(entries: Vec<(&str, MergedValue)>) -> MergedValue {
        MergedValue::Object(
            MergedObject::new(
                entries
                    .into_iter()
                    .map(|(key, value)| MergedObjectEntry {
                        name: key.intern().to::<ValueKeyName>(),
                        value,
                    })
                    .collect(),
            )
            .expect("the test passes each key once"),
        )
    }

    fn key(s: &str, list: Vec<MergedArgument>) -> SelectionKey {
        SelectionKey {
            name: name(s),
            arguments: arguments(list),
        }
    }

    fn leaf(s: &str, list: Vec<MergedArgument>) -> MergedSelection {
        MergedSelection::Selectable(MergedSelectableSelection {
            key: key(s, list),
            target: MergedSelectionTarget::Leaf,
        })
    }

    fn composite(
        s: &str,
        list: Vec<MergedArgument>,
        identity: IdentitySelection,
        selections: Vec<MergedSelection>,
    ) -> MergedSelection {
        MergedSelection::Selectable(MergedSelectableSelection {
            key: key(s, list),
            target: MergedSelectionTarget::Composite(MergedCompositeTarget {
                identity,
                selections: map(selections),
            }),
        })
    }

    fn conditional(
        on: &str,
        identity: IdentitySelection,
        selections: Vec<MergedSelection>,
    ) -> MergedSelection {
        MergedSelection::Conditional(MergedConditional {
            condition: Condition::SelectionEquals(entity(on)),
            identity,
            selections: map(selections),
        })
    }

    fn try_map(selections: Vec<MergedSelection>) -> Result<MergedSelectionMap, MergeError> {
        selections
            .into_iter()
            .try_fold(MergedSelectionMap::empty(), |map, selection| {
                map.merge(MergedSelectionMap::single(selection))
            })
    }

    fn map(selections: Vec<MergedSelection>) -> MergedSelectionMap {
        try_map(selections).expect("the test's selections merge")
    }

    fn id() -> IdentitySelection {
        IdentitySelection::Select(name("id"))
    }

    fn aliases(map: &MergedSelectionMap) -> Vec<String> {
        map.iter()
            .map(|selection| match selection {
                MergedSelection::Selectable(selection) => selection.key.normalization_alias().0,
                MergedSelection::Conditional(conditional) => match conditional.condition {
                    Condition::SelectionEquals(on) => format!("... on {on}"),
                },
            })
            .collect()
    }

    fn nested<'a>(map: &'a MergedSelectionMap, s: &str) -> &'a MergedSelectionMap {
        map.iter()
            .find_map(|selection| match selection {
                MergedSelection::Selectable(selection) if selection.key.name == name(s) => {
                    match selection.target.reference() {
                        MergedSelectionTarget::Composite(target) => {
                            target.selections.reference().wrap_some()
                        }
                        MergedSelectionTarget::Leaf => None,
                    }
                }
                MergedSelection::Selectable(_) | MergedSelection::Conditional(_) => None,
            })
            .expect("the test's map has this composite selection")
    }

    #[test]
    fn arguments_are_sorted_by_name_and_a_repeated_name_is_rejected() {
        let sorted = MergedArguments::new(vec![
            argument("b", MergedValue::Integer(2)),
            argument("a", MergedValue::Integer(1)),
        ])
        .expect("distinct names");
        assert_eq!(
            sorted.iter().map(|a| a.name).collect::<Vec<_>>(),
            vec![argument_name("a"), argument_name("b")]
        );
        assert_eq!(
            MergedArguments::new(vec![
                argument("a", MergedValue::Integer(1)),
                argument("a", MergedValue::Integer(2)),
            ]),
            argument_name("a").wrap_err()
        );
        assert!(MergedArguments::new(Vec::new()).expect("empty").is_empty());
        assert_eq!(
            MergedArguments::new(Vec::new()),
            MergedArguments::default().wrap_ok()
        );
    }

    #[test]
    fn object_entries_are_sorted_by_name_and_a_repeated_name_is_rejected() {
        let entry = |key: &str| MergedObjectEntry {
            name: key.intern().to(),
            value: MergedValue::Null,
        };
        let sorted = MergedObject::new(vec![entry("z"), entry("a")]).expect("distinct keys");
        assert_eq!(
            sorted.iter().map(|e| e.name).collect::<Vec<_>>(),
            vec!["a".intern().to::<ValueKeyName>(), "z".intern().to()]
        );
        assert_eq!(
            MergedObject::new(vec![entry("k"), entry("k")]),
            "k".intern().to::<ValueKeyName>().wrap_err()
        );
    }

    #[test]
    fn normalization_aliases_name_every_argument_value() {
        let alias = |key: SelectionKey| key.normalization_alias().0;
        assert_eq!(alias(key("firstName", vec![])), "firstName");
        assert_eq!(
            alias(key("pet", vec![argument("id", var("id"))])),
            "pet____id___v_id"
        );
        assert_eq!(
            alias(key(
                "checkins",
                vec![
                    argument("skip", MergedValue::Integer(0)),
                    argument("limit", MergedValue::Integer(5)),
                ]
            )),
            "checkins____limit___l_5____skip___l_0"
        );
        assert_eq!(
            alias(key(
                "set_pet_tagline",
                vec![argument(
                    "input",
                    object(vec![("tagline", var("tagline")), ("id", var("id"))])
                )]
            )),
            "set_pet_tagline____input___o_id__v_id_tagline__v_tagline_c"
        );
        assert_eq!(
            alias(key(
                "f",
                vec![
                    argument("a", MergedValue::Boolean(Boolean::True)),
                    argument("b", MergedValue::Boolean(Boolean::False)),
                    argument("c", MergedValue::Null),
                    argument("d", MergedValue::Integer(-3)),
                ]
            )),
            "f____a___l_true____b___l_false____c___l_null____d___l_-3"
        );
        assert_eq!(
            alias(key("f", vec![argument("s", string("a b-c.Ü_1"))])),
            "f____s___s_a_b_c___1"
        );
        assert_eq!(
            alias(key(
                "f",
                vec![argument(
                    "o",
                    object(vec![(
                        "inner",
                        object(vec![("x", MergedValue::Integer(1))])
                    )])
                )]
            )),
            "f____o___o_inner__o_x__l_1_c_c"
        );
        assert_eq!(
            alias(key("f", vec![argument("o", object(vec![]))])),
            "f____o___o__c"
        );
    }

    #[test]
    fn an_identical_selection_merges_into_one_entry() {
        let merged = map(vec![
            leaf("id", vec![]),
            leaf("id", vec![]),
            leaf("age", vec![]),
        ]);
        assert_eq!(aliases(merged.reference()), vec!["age", "id"]);
    }

    #[test]
    fn argument_order_does_not_change_the_key() {
        let merged = map(vec![
            leaf(
                "f",
                vec![
                    argument("a", MergedValue::Integer(1)),
                    argument("b", MergedValue::Integer(2)),
                ],
            ),
            leaf(
                "f",
                vec![
                    argument("b", MergedValue::Integer(2)),
                    argument("a", MergedValue::Integer(1)),
                ],
            ),
        ]);
        assert_eq!(aliases(merged.reference()), vec!["f____a___l_1____b___l_2"]);
    }

    #[test]
    fn different_arguments_to_one_selectable_are_two_entries() {
        let merged = map(vec![
            leaf("checkins", vec![argument("limit", MergedValue::Integer(3))]),
            leaf("checkins", vec![argument("limit", MergedValue::Integer(5))]),
            leaf("checkins", vec![]),
        ]);
        assert_eq!(
            aliases(merged.reference()),
            vec![
                "checkins",
                "checkins____limit___l_3",
                "checkins____limit___l_5"
            ]
        );
    }

    #[test]
    fn composite_selections_of_one_key_merge_their_selections_recursively() {
        let merged = map(vec![
            composite(
                "pet",
                vec![],
                id(),
                vec![composite(
                    "stats",
                    vec![],
                    IdentitySelection::Omit,
                    vec![leaf("weight", vec![])],
                )],
            ),
            composite(
                "pet",
                vec![],
                id(),
                vec![
                    leaf("age", vec![]),
                    composite(
                        "stats",
                        vec![],
                        IdentitySelection::Omit,
                        vec![leaf("energy", vec![])],
                    ),
                ],
            ),
        ]);
        assert_eq!(aliases(merged.reference()), vec!["pet"]);
        let pet = nested(merged.reference(), "pet");
        assert_eq!(aliases(pet), vec!["age", "stats"]);
        assert_eq!(aliases(nested(pet, "stats")), vec!["energy", "weight"]);
    }

    #[test]
    fn merging_is_independent_of_order() {
        let first = || {
            map(vec![
                composite("pet", vec![], id(), vec![leaf("age", vec![])]),
                leaf("b", vec![]),
            ])
        };
        let second = || {
            map(vec![
                composite("pet", vec![], id(), vec![leaf("name", vec![])]),
                leaf("a", vec![]),
                conditional("Pet", id(), vec![leaf("x", vec![])]),
            ])
        };
        assert_eq!(first().merge(second()), second().merge(first()));
    }

    #[test]
    fn merging_with_the_empty_map_changes_nothing() {
        let selections = || map(vec![leaf("a", vec![]), conditional("T", id(), vec![])]);
        assert_eq!(
            selections().merge(MergedSelectionMap::empty()),
            selections().wrap_ok()
        );
        assert_eq!(
            MergedSelectionMap::empty().merge(selections()),
            selections().wrap_ok()
        );
        assert_eq!(
            MergedSelectionMap::empty().merge(MergedSelectionMap::empty()),
            MergedSelectionMap::empty().wrap_ok()
        );
        assert_eq!(MergedSelectionMap::empty().iter().count(), 0);
    }

    #[test]
    fn a_leaf_and_a_composite_at_one_key_are_a_mismatch() {
        assert_eq!(
            try_map(vec![
                leaf("pet", vec![]),
                composite("pet", vec![], id(), vec![])
            ]),
            MergeError::SelectionMismatch.wrap_err()
        );
        assert_eq!(
            try_map(vec![
                composite("pet", vec![], id(), vec![]),
                leaf("pet", vec![])
            ]),
            MergeError::SelectionMismatch.wrap_err()
        );
    }

    #[test]
    fn composites_or_conditionals_with_different_identities_are_a_mismatch() {
        assert_eq!(
            try_map(vec![
                composite("pet", vec![], id(), vec![]),
                composite("pet", vec![], IdentitySelection::Omit, vec![]),
            ]),
            MergeError::SelectionMismatch.wrap_err()
        );
        assert_eq!(
            try_map(vec![
                conditional("T", id(), vec![]),
                conditional("T", IdentitySelection::Omit, vec![]),
            ]),
            MergeError::SelectionMismatch.wrap_err()
        );
    }

    #[test]
    fn a_mismatch_below_the_top_is_a_mismatch_of_the_map() {
        assert_eq!(
            try_map(vec![
                composite("pet", vec![], id(), vec![leaf("stats", vec![])]),
                composite(
                    "pet",
                    vec![],
                    id(),
                    vec![composite("stats", vec![], IdentitySelection::Omit, vec![])]
                ),
            ]),
            MergeError::SelectionMismatch.wrap_err()
        );
    }

    #[test]
    fn conditionals_on_one_entity_merge_and_on_two_entities_do_not() {
        let merged = map(vec![
            conditional("BlogItem", id(), vec![leaf("title", vec![])]),
            conditional("AdItem", id(), vec![leaf("message", vec![])]),
            conditional("BlogItem", id(), vec![leaf("author", vec![])]),
            leaf("z", vec![]),
        ]);
        assert_eq!(
            aliases(merged.reference()),
            vec!["z", "... on AdItem", "... on BlogItem"]
        );
        let blog = merged
            .iter()
            .find_map(|selection| match selection {
                MergedSelection::Conditional(conditional)
                    if conditional.condition == Condition::SelectionEquals(entity("BlogItem")) =>
                {
                    conditional.selections.reference().wrap_some()
                }
                MergedSelection::Selectable(_) | MergedSelection::Conditional(_) => None,
            })
            .expect("the map has the BlogItem conditional");
        assert_eq!(aliases(blog), vec!["author", "title"]);
    }

    #[test]
    fn two_keys_with_one_alias_collide() {
        assert_eq!(
            try_map(vec![
                leaf("f", vec![argument("s", string("a b"))]),
                leaf("f", vec![argument("s", string("a_b"))]),
            ]),
            MergeError::AliasCollision(AliasCollision {
                first: key("f", vec![argument("s", string("a b"))]),
                second: key("f", vec![argument("s", string("a_b"))]),
            })
            .wrap_err()
        );
        assert_eq!(
            try_map(vec![
                leaf("f", vec![argument("s", string("a"))]),
                leaf("f____s___s_a", vec![]),
            ])
            .map(|_| ()),
            MergeError::AliasCollision(AliasCollision {
                first: key("f", vec![argument("s", string("a"))]),
                second: key("f____s___s_a", vec![]),
            })
            .wrap_err()
        );
    }

    #[test]
    fn a_collision_in_a_nested_map_fails_the_merge() {
        assert!(matches!(
            try_map(vec![
                composite(
                    "pet",
                    vec![],
                    id(),
                    vec![leaf("f", vec![argument("s", string("a b"))])]
                ),
                composite(
                    "pet",
                    vec![],
                    id(),
                    vec![leaf("f", vec![argument("s", string("a_b"))])]
                ),
            ]),
            Err(MergeError::AliasCollision(_))
        ));
    }

    #[test]
    fn a_conditional_has_no_alias_and_does_not_collide() {
        let merged = map(vec![leaf("T", vec![]), conditional("T", id(), vec![])]);
        assert_eq!(aliases(merged.reference()), vec!["T", "... on T"]);
    }

    #[test]
    fn the_key_of_a_selection_is_its_selection_key_or_its_condition() {
        assert_eq!(
            leaf("a", vec![argument("x", MergedValue::Integer(1))]).key(),
            MergeKey::Selectable(key("a", vec![argument("x", MergedValue::Integer(1))]))
        );
        assert_eq!(
            conditional("T", id(), vec![]).key(),
            MergeKey::Conditional(Condition::SelectionEquals(entity("T")))
        );
    }

    #[test]
    fn iteration_is_alphabetical_by_name_then_arguments_then_conditionals() {
        let merged = map(vec![
            conditional("A", id(), vec![]),
            leaf("b", vec![]),
            leaf("a", vec![argument("x", MergedValue::Integer(2))]),
            leaf("a", vec![]),
            leaf("B", vec![]),
        ]);
        assert_eq!(
            aliases(merged.reference()),
            vec!["B", "a", "a____x___l_2", "b", "... on A"]
        );
    }

    #[test]
    fn used_variables_are_found_at_every_depth() {
        let merged = map(vec![
            leaf("a", vec![argument("x", var("top"))]),
            composite(
                "b",
                vec![argument("o", object(vec![("k", var("in_object"))]))],
                id(),
                vec![leaf("c", vec![argument("y", var("nested"))])],
            ),
            conditional(
                "T",
                id(),
                vec![leaf("d", vec![argument("z", var("conditional"))])],
            ),
            leaf("e", vec![argument("w", MergedValue::Integer(1))]),
        ]);
        assert_eq!(
            merged.used_variables(),
            ["top", "in_object", "nested", "conditional"]
                .into_iter()
                .map(variable)
                .collect::<BTreeSet<_>>()
        );
        assert_eq!(
            MergedSelectionMap::empty().used_variables(),
            BTreeSet::new()
        );
        assert_eq!(
            map(vec![leaf("a", vec![])]).used_variables(),
            BTreeSet::new()
        );
    }

    fn context(values: Vec<(&str, VariableValue)>) -> BTreeMap<VariableName, VariableValue> {
        values
            .into_iter()
            .map(|(name, value)| (variable(name), value))
            .collect()
    }

    #[test]
    fn substitution_replaces_each_variable_with_its_value() {
        let field = map(vec![leaf(
            "checkins",
            vec![argument("limit", var("limit"))],
        )]);
        assert_eq!(
            substitute(
                field.reference(),
                context(vec![(
                    "limit",
                    VariableValue::Value(MergedValue::Integer(5))
                )])
                .reference()
            ),
            map(vec![leaf(
                "checkins",
                vec![argument("limit", MergedValue::Integer(5))]
            )])
            .wrap_ok()
        );
        assert_eq!(
            substitute(
                field.reference(),
                context(vec![("limit", VariableValue::Value(var("parent")))]).reference()
            ),
            map(vec![leaf(
                "checkins",
                vec![argument("limit", var("parent"))]
            )])
            .wrap_ok()
        );
    }

    #[test]
    fn an_unset_variable_removes_its_argument_and_is_null_inside_an_object() {
        let field = map(vec![
            leaf(
                "checkins",
                vec![
                    argument("limit", var("x")),
                    argument("skip", MergedValue::Integer(0)),
                ],
            ),
            leaf(
                "f",
                vec![argument(
                    "input",
                    object(vec![("a", var("x")), ("b", MergedValue::Integer(1))]),
                )],
            ),
        ]);
        assert_eq!(
            substitute(
                field.reference(),
                context(vec![("x", VariableValue::Unset)]).reference()
            ),
            map(vec![
                leaf("checkins", vec![argument("skip", MergedValue::Integer(0))]),
                leaf(
                    "f",
                    vec![argument(
                        "input",
                        object(vec![
                            ("a", MergedValue::Null),
                            ("b", MergedValue::Integer(1))
                        ])
                    )]
                ),
            ])
            .wrap_ok()
        );
    }

    #[test]
    fn substitution_reaches_composite_targets_and_conditionals() {
        let field = map(vec![composite(
            "pet",
            vec![argument("id", var("id"))],
            id(),
            vec![conditional(
                "T",
                id(),
                vec![leaf("c", vec![argument("n", var("id"))])],
            )],
        )]);
        assert_eq!(
            substitute(
                field.reference(),
                context(vec![("id", VariableValue::Value(string("1")))]).reference()
            ),
            map(vec![composite(
                "pet",
                vec![argument("id", string("1"))],
                id(),
                vec![conditional(
                    "T",
                    id(),
                    vec![leaf("c", vec![argument("n", string("1"))])]
                )],
            )])
            .wrap_ok()
        );
    }

    #[test]
    fn keys_that_become_equal_merge() {
        let field = map(vec![
            composite(
                "pet",
                vec![argument("id", var("a"))],
                id(),
                vec![leaf("x", vec![])],
            ),
            composite(
                "pet",
                vec![argument("id", var("b"))],
                id(),
                vec![leaf("y", vec![])],
            ),
        ]);
        let one = || VariableValue::Value(MergedValue::Integer(1));
        let substituted = substitute(
            field.reference(),
            context(vec![("a", one()), ("b", one())]).reference(),
        )
        .expect("equal keys merge");
        assert_eq!(aliases(substituted.reference()), vec!["pet____id___l_1"]);
        assert_eq!(
            aliases(nested(substituted.reference(), "pet")),
            vec!["x", "y"]
        );
        let both_unset = substitute(
            field.reference(),
            context(vec![
                ("a", VariableValue::Unset),
                ("b", VariableValue::Unset),
            ])
            .reference(),
        )
        .expect("equal keys merge");
        assert_eq!(aliases(both_unset.reference()), vec!["pet"]);
    }

    #[test]
    fn substitution_that_makes_aliases_equal_is_a_collision() {
        let field = map(vec![
            leaf("f", vec![argument("s", var("a"))]),
            leaf("f", vec![argument("s", var("b"))]),
        ]);
        assert!(matches!(
            substitute(
                field.reference(),
                context(vec![
                    ("a", VariableValue::Value(string("a b"))),
                    ("b", VariableValue::Value(string("a_b"))),
                ])
                .reference()
            ),
            Err(SelectionMapErrorKind::Merge(MergeError::AliasCollision(_)))
        ));
    }

    #[test]
    fn a_variable_missing_from_the_context_is_undefined() {
        let field = map(vec![leaf(
            "f",
            vec![argument("s", object(vec![("k", var("missing"))]))],
        )]);
        assert_eq!(
            substitute(field.reference(), context(vec![]).reference()),
            SelectionMapErrorKind::UndefinedVariable(variable("missing")).wrap_err()
        );
    }

    #[test]
    fn literals_and_the_empty_map_are_unchanged_by_substitution() {
        let field = map(vec![leaf(
            "f",
            vec![
                argument("a", MergedValue::Boolean(Boolean::True)),
                argument("b", MergedValue::Null),
                argument("c", string("s")),
            ],
        )]);
        assert_eq!(
            substitute(field.reference(), context(vec![]).reference()),
            field.clone().wrap_ok()
        );
        assert_eq!(
            substitute(
                MergedSelectionMap::empty().reference(),
                context(vec![]).reference()
            ),
            MergedSelectionMap::empty().wrap_ok()
        );
    }
}
