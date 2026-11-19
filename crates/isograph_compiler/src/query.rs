use common_lang_types::{ArtifactFileName, Diagnostic, EntityName, QueryText, SelectableName};
use pico_macros::memo;
use prelude::Postfix;

use crate::merged_selection::{
    CompletedSelectionMap, FieldSelectionMap, FieldVariable, SelectionMapError, TopLevelError,
    field_selection_map,
};
use crate::validation::EntrypointNotFetchableError;
use crate::{
    ClientId, CompilationProfile, IsographDb, NetworkProtocol, entity_named, entrypoint_fetch_plan,
    selectable_named,
};

/// What the core hands the protocol to print one entrypoint's query.
pub struct EntrypointOperation<'a, TProtocol: NetworkProtocol> {
    /// The operation name: the entrypoint's selectable name.
    pub name: SelectableName,
    pub fetch_plan: &'a TProtocol::FetchPlan,
    /// The entrypoint field's variables that the selections use, in declaration order.
    pub variables: &'a [FieldVariable],
    pub selections: &'a CompletedSelectionMap,
}

/// The text the protocol prints for one operation, and its name for a file holding exactly
/// that text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolQueryText {
    pub text: QueryText,
    pub file_name: Option<ArtifactFileName>,
}

/// One entrypoint's query: its text and the protocol's file name for the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryArtifact {
    pub text: QueryText,
    pub file_name: Option<ArtifactFileName>,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EntrypointQueryError<TNotFetchable: std::fmt::Display> {
    #[error("{}", .0.0.message)]
    Lookup(Diagnostic),
    #[error("{0}")]
    NotFetchable(EntrypointNotFetchableError<TNotFetchable>),
    #[error("{0}")]
    SelectionMap(SelectionMapError),
    #[error("{0}")]
    TopLevel(TopLevelError),
}

pub type ProfileEntrypointQueryError<TProfile> = EntrypointQueryError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;

/// The query of `entrypoint entity.selectable` as `client` reads the source and schema.
#[memo]
pub fn query_for_entrypoint<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: EntityName,
    selectable: SelectableName,
) -> Result<QueryArtifact, ProfileEntrypointQueryError<TProfile>> {
    let entity_value = entity_named(db, client, entity)
        .clone()
        .map_err(EntrypointQueryError::Lookup)?;
    let selectable_value = selectable_named(db, client, entity, selectable)
        .clone()
        .map_err(EntrypointQueryError::Lookup)?;
    let fetch_plan = entrypoint_fetch_plan(
        db,
        client,
        entity_value.reference(),
        selectable_value.reference(),
    )
    .map_err(EntrypointQueryError::NotFetchable)?;
    let field = field_selection_map(db, client, entity, selectable)
        .as_ref()
        .map_err(|error| EntrypointQueryError::SelectionMap(error.clone()))?;
    let variables = operation_variables(field);
    let selections =
        CompletedSelectionMap::top_level::<TProfile::NetworkProtocol>(field.selections.reference())
            .map_err(EntrypointQueryError::TopLevel)?;
    let ProtocolQueryText { text, file_name } =
        TProfile::NetworkProtocol::query_text(&EntrypointOperation {
            name: selectable,
            fetch_plan: fetch_plan.reference(),
            variables: variables.as_slice(),
            selections: selections.reference(),
        });
    QueryArtifact { text, file_name }.wrap_ok()
}

/// The entrypoint field's variables that its selections use, in declaration order.
fn operation_variables(field: &FieldSelectionMap) -> Vec<FieldVariable> {
    let used = field.selections.used_variables();
    field
        .variables
        .iter()
        .filter(|variable| used.contains(variable.name.reference()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use common_lang_types::{EntityName, VariableName};
    use intern::string_key::Intern;
    use prelude::Postfix;

    use super::operation_variables;
    use crate::merged_selection::{
        FieldSelectionMap, FieldVariable, MergedArgument, MergedArguments,
        MergedSelectableSelection, MergedSelection, MergedSelectionMap, MergedSelectionTarget,
        MergedValue, NonNullVariableType, SelectionKey, VariableType,
    };

    fn variable(name: &str) -> VariableName {
        name.intern().to()
    }

    fn field_variable(name: &str) -> FieldVariable {
        FieldVariable {
            name: variable(name),
            variable_type: VariableType::NonNull(NonNullVariableType::Named(
                "ID".intern().to::<EntityName>(),
            )),
            default_value: None,
        }
    }

    /// A leaf selection of `name` whose one argument `a` is the variable `uses`.
    fn using(name: &str, uses: &str) -> MergedSelectionMap {
        MergedSelectionMap::single(MergedSelection::Selectable(MergedSelectableSelection {
            key: SelectionKey {
                name: name.intern().to(),
                arguments: MergedArguments::new(
                    MergedArgument {
                        name: "a".intern().to(),
                        value: MergedValue::Variable(variable(uses)),
                    }
                    .wrap_vec(),
                )
                .expect("one argument"),
            },
            target: MergedSelectionTarget::Leaf,
        }))
    }

    fn field(variables: &[&str], uses: &[&str]) -> FieldSelectionMap {
        FieldSelectionMap {
            variables: variables.iter().map(|name| field_variable(name)).collect(),
            selections: uses
                .iter()
                .enumerate()
                .map(|(index, uses)| using(format!("f{index}").as_str(), uses))
                .try_fold(MergedSelectionMap::empty(), MergedSelectionMap::merge)
                .expect("distinct selections"),
        }
    }

    fn names(variables: Vec<FieldVariable>) -> Vec<VariableName> {
        variables
            .into_iter()
            .map(|variable| variable.name)
            .collect()
    }

    #[test]
    fn only_used_variables_are_declared_in_declaration_order() {
        assert_eq!(
            names(operation_variables(
                field(&["b", "unused", "a"], &["a", "b"]).reference()
            )),
            vec![variable("b"), variable("a")]
        );
        assert_eq!(
            names(operation_variables(field(&["x"], &[]).reference())),
            Vec::<VariableName>::new()
        );
        assert_eq!(
            names(operation_variables(field(&[], &[]).reference())),
            Vec::<VariableName>::new()
        );
    }

    #[test]
    fn a_variable_used_twice_is_declared_once() {
        assert_eq!(
            names(operation_variables(field(&["a"], &["a", "a"]).reference())),
            vec![variable("a")]
        );
    }
}
