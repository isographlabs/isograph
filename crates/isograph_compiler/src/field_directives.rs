use common_lang_types::{IsographDirectiveName, WithLocation, WithSpan};
use intern::string_key::Intern;
use isograph_lang_types::{
    ClientFieldDeclaration, ClientPointerDeclaration, LinkedFieldSelection, ScalarFieldSelection,
    ServerFieldSelection, UnvalidatedSelection, UnvalidatedSelectionWithUnvalidatedDirectives,
};
use isograph_schema::ProcessClientFieldDeclarationError;
use lazy_static::lazy_static;

lazy_static! {
    static ref LOADABLE_DIRECTIVE_NAME: IsographDirectiveName = "loadable".intern().into();
    static ref UPDATABLE_DIRECTIVE_NAME: IsographDirectiveName = "updatable".intern().into();
}

#[allow(clippy::complexity)]
pub fn validate_isograph_field_directives(
    client_field: WithSpan<ClientFieldDeclaration>,
) -> Result<WithSpan<ClientFieldDeclaration>, Vec<WithLocation<ProcessClientFieldDeclarationError>>>
{
    client_field.and_then(|client_field| {
        let ClientFieldDeclaration {
            const_export_name,
            parent_type,
            client_field_name,
            description,
            selection_set,
            directives,
            variable_definitions,
            definition_path,
            dot,
            field_keyword,
        } = client_field;

        Ok(ClientFieldDeclaration {
            const_export_name,
            parent_type,
            client_field_name,
            description,
            selection_set: validate_isograph_selection_set_directives(selection_set)?,
            directives,
            variable_definitions,
            definition_path,
            dot,
            field_keyword,
        })
    })
}

pub fn validate_isograph_selection_set_directives(
    selection_set: Vec<WithSpan<UnvalidatedSelectionWithUnvalidatedDirectives>>,
) -> Result<
    Vec<WithSpan<UnvalidatedSelection>>,
    Vec<WithLocation<ProcessClientFieldDeclarationError>>,
> {
    and_then_selection_set_and_collect_errors(
        selection_set,
        &|scalar_field_selection| Ok(scalar_field_selection.associated_data),
        &|linked_field_selection| Ok(linked_field_selection.associated_data),
    )
}

#[allow(clippy::complexity)]
pub fn validate_isograph_pointer_directives(
    client_pointer: WithSpan<ClientPointerDeclaration>,
) -> Result<WithSpan<ClientPointerDeclaration>, Vec<WithLocation<ProcessClientFieldDeclarationError>>>
{
    client_pointer.and_then(|client_pointer| {
        let ClientPointerDeclaration {
            const_export_name,
            parent_type,
            client_pointer_name,
            description,
            selection_set,
            variable_definitions,
            definition_path,
            dot,
            pointer_keyword,
            target_type,
            directives,
        } = client_pointer;

        Ok(ClientPointerDeclaration {
            const_export_name,
            parent_type,
            client_pointer_name,
            description,
            selection_set: validate_isograph_selection_set_directives(selection_set)?,
            variable_definitions,
            definition_path,
            dot,
            pointer_keyword,
            target_type,
            directives,
        })
    })
}

fn and_then_selection_set_and_collect_errors<
    TScalarField,
    TLinkedField,
    TScalarField2,
    TLinkedField2,
    E,
>(
    selection_set: Vec<WithSpan<ServerFieldSelection<TScalarField, TLinkedField>>>,
    and_then_scalar: &impl Fn(&ScalarFieldSelection<TScalarField>) -> Result<TScalarField2, E>,
    and_then_linked: &impl Fn(
        &LinkedFieldSelection<TScalarField, TLinkedField>,
    ) -> Result<TLinkedField2, E>,
) -> Result<Vec<WithSpan<ServerFieldSelection<TScalarField2, TLinkedField2>>>, Vec<E>> {
    let mut errors = vec![];
    let mut transformed_selection_set = vec![];

    for with_span in selection_set {
        match with_span.item {
            ServerFieldSelection::LinkedField(l) => {
                let new_linked_field_data = and_then_linked(&l);
                match new_linked_field_data {
                    Ok(new_linked_field) => {
                        let sub_errors = and_then_selection_set_and_collect_errors(
                            l.selection_set,
                            and_then_scalar,
                            and_then_linked,
                        );
                        match sub_errors {
                            Ok(new_selection_set) => transformed_selection_set.push(WithSpan::new(
                                ServerFieldSelection::LinkedField(LinkedFieldSelection {
                                    name: l.name,
                                    reader_alias: l.reader_alias,
                                    associated_data: new_linked_field,
                                    selection_set: new_selection_set,
                                    arguments: l.arguments,
                                }),
                                with_span.span,
                            )),
                            Err(e) => errors.extend(e),
                        }
                    }
                    Err(e) => errors.push(e),
                }
            }
            ServerFieldSelection::ScalarField(s) => {
                match and_then_scalar(&s) {
                    Ok(new_scalar_field_data) => transformed_selection_set.push(WithSpan::new(
                        ServerFieldSelection::ScalarField(ScalarFieldSelection {
                            name: s.name,
                            reader_alias: s.reader_alias,
                            associated_data: new_scalar_field_data,
                            arguments: s.arguments,
                        }),
                        with_span.span,
                    )),
                    Err(e) => errors.push(e),
                };
            }
        };
    }

    if errors.is_empty() {
        Ok(transformed_selection_set)
    } else {
        Err(errors)
    }
}
