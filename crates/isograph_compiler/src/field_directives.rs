use common_lang_types::{IsographDirectiveName, Location, WithLocation, WithSpan};
use intern::string_key::Intern;
use isograph_lang_types::{
    from_isograph_field_directive, ClientFieldDeclaration,
    ClientFieldDeclarationWithUnvalidatedDirectives, ClientFieldDeclarationWithValidatedDirectives,
    ClientPointerDeclaration, ClientPointerDeclarationWithUnvalidatedDirectives,
    ClientPointerDeclarationWithValidatedDirectives, IsographFieldDirective,
    IsographSelectionVariant, LinkedFieldSelection, ScalarFieldSelection, Selection,
    ServerFieldSelection, UnvalidatedSelection,
};
use isograph_schema::ProcessClientFieldDeclarationError;
use lazy_static::lazy_static;

lazy_static! {
    static ref LOADABLE_DIRECTIVE_NAME: IsographDirectiveName = "loadable".intern().into();
}

#[allow(clippy::complexity)]
pub fn validate_isograph_field_directives(
    client_field: WithSpan<ClientFieldDeclarationWithUnvalidatedDirectives>,
) -> Result<
    WithSpan<ClientFieldDeclarationWithValidatedDirectives>,
    Vec<WithLocation<ProcessClientFieldDeclarationError>>,
> {
    client_field.and_then(|client_field| {
        let ClientFieldDeclaration {
            const_export_name,
            parent_type,
            client_field_name,
            description,
            selection_set,
            unwraps,
            directives,
            variable_definitions,
            definition_path,
            dot,
            field_keyword,
        } = client_field;

        Ok(ClientFieldDeclarationWithValidatedDirectives {
            const_export_name,
            parent_type,
            client_field_name,
            description,
            selection_set: validate_isograph_selection_set_directives(selection_set)?,
            unwraps,
            directives,
            variable_definitions,
            definition_path,
            dot,
            field_keyword,
        })
    })
}

pub fn validate_isograph_selection_set_directives(
    selection_set: Vec<WithSpan<Selection<(), ()>>>,
) -> Result<
    Vec<WithSpan<UnvalidatedSelection>>,
    Vec<WithLocation<ProcessClientFieldDeclarationError>>,
> {
    and_then_selection_set_and_collect_errors(
        selection_set,
        &|scalar_field_selection| {
            if let Some(directive) =
                find_directive_named(&scalar_field_selection.directives, *LOADABLE_DIRECTIVE_NAME)
            {
                let loadable_variant =
                    from_isograph_field_directive(&directive.item).map_err(|message| {
                        WithLocation::new(
                            ProcessClientFieldDeclarationError::UnableToDeserialize {
                                directive_name: *LOADABLE_DIRECTIVE_NAME,
                                message,
                            },
                            Location::generated(),
                        )
                    })?;
                // TODO validate that the field is actually loadable (i.e. implements Node or
                // whatnot)
                Ok(IsographSelectionVariant::Loadable(loadable_variant))
            } else {
                Ok(IsographSelectionVariant::Regular)
            }
        },
        &|_object_pointer_selection| Ok(IsographSelectionVariant::Regular),
    )
}

#[allow(clippy::complexity)]
pub fn validate_isograph_pointer_directives(
    client_pointer: WithSpan<ClientPointerDeclarationWithUnvalidatedDirectives>,
) -> Result<
    WithSpan<ClientPointerDeclarationWithValidatedDirectives>,
    Vec<WithLocation<ProcessClientFieldDeclarationError>>,
> {
    client_pointer.and_then(|client_pointer| {
        let ClientPointerDeclaration {
            const_export_name,
            parent_type,
            client_pointer_name,
            description,
            selection_set,
            unwraps,
            variable_definitions,
            definition_path,
            dot,
            pointer_keyword,
            to_type,
        } = client_pointer;

        Ok(ClientPointerDeclarationWithValidatedDirectives {
            const_export_name,
            parent_type,
            client_pointer_name,
            description,
            selection_set: validate_isograph_selection_set_directives(selection_set)?,
            unwraps,
            variable_definitions,
            definition_path,
            dot,
            pointer_keyword,
            to_type,
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
    selection_set: Vec<WithSpan<Selection<TScalarField, TLinkedField>>>,
    and_then_scalar: &impl Fn(&ScalarFieldSelection<TScalarField>) -> Result<TScalarField2, E>,
    and_then_linked: &impl Fn(
        &LinkedFieldSelection<TScalarField, TLinkedField>,
    ) -> Result<TLinkedField2, E>,
) -> Result<Vec<WithSpan<Selection<TScalarField2, TLinkedField2>>>, Vec<E>> {
    let mut errors = vec![];
    let mut transformed_selection_set = vec![];

    for with_span in selection_set {
        match with_span.item {
            Selection::ServerField(ServerFieldSelection::LinkedField(l)) => {
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
                                Selection::ServerField(ServerFieldSelection::LinkedField(
                                    LinkedFieldSelection {
                                        name: l.name,
                                        reader_alias: l.reader_alias,
                                        associated_data: new_linked_field,
                                        selection_set: new_selection_set,
                                        unwraps: l.unwraps,
                                        arguments: l.arguments,
                                        directives: l.directives,
                                    },
                                )),
                                with_span.span,
                            )),
                            Err(e) => errors.extend(e),
                        }
                    }
                    Err(e) => errors.push(e),
                }
            }
            Selection::ServerField(ServerFieldSelection::ScalarField(s)) => {
                match and_then_scalar(&s) {
                    Ok(new_scalar_field_data) => transformed_selection_set.push(WithSpan::new(
                        Selection::ServerField(ServerFieldSelection::ScalarField(
                            ScalarFieldSelection {
                                name: s.name,
                                reader_alias: s.reader_alias,
                                associated_data: new_scalar_field_data,
                                unwraps: s.unwraps,
                                arguments: s.arguments,
                                directives: s.directives,
                            },
                        )),
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

fn find_directive_named(
    directives: &[WithSpan<IsographFieldDirective>],
    name: IsographDirectiveName,
) -> Option<&WithSpan<IsographFieldDirective>> {
    directives
        .iter()
        .find(|directive| directive.item.name.item == name)
}
