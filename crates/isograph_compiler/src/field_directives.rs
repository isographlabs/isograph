use common_lang_types::{IsographDirectiveName, Location, WithLocation, WithSpan};
use intern::string_key::Intern;
use isograph_lang_types::{
    from_isograph_field_directive, ClientFieldDeclaration,
    ClientFieldDeclarationWithUnvalidatedDirectives, ClientFieldDeclarationWithValidatedDirectives,
    IsographFieldDirective, IsographSelectionVariant, LinkedFieldSelection, ScalarFieldSelection,
    ServerFieldSelection,
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
    } = client_field.item;
    let new_selecton_set = and_then_selection_set_and_collect_errors(
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
        &|_linked_field_selection| Ok(IsographSelectionVariant::Regular),
    )?;
    Ok(WithSpan::new(
        ClientFieldDeclarationWithValidatedDirectives {
            const_export_name,
            parent_type,
            client_field_name,
            description,
            selection_set: new_selecton_set,
            directives,
            variable_definitions,
            definition_path,
            dot,
            field_keyword,
        },
        client_field.span,
    ))
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
                                    directives: l.directives,
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
                            directives: s.directives,
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

fn find_directive_named(
    directives: &[WithSpan<IsographFieldDirective>],
    name: IsographDirectiveName,
) -> Option<&WithSpan<IsographFieldDirective>> {
    directives
        .iter()
        .find(|directive| directive.item.name.item == name)
}
