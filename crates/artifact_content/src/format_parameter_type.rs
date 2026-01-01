use isograph_lang_types::{TypeAnnotationDeclaration, UnionVariant};
use isograph_schema::{CompilationProfile, IsographDatabase, TargetPlatform};
use prelude::Postfix;

pub(crate) fn format_parameter_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    type_: &TypeAnnotationDeclaration,
    indentation_level: u8,
) -> String {
    match type_ {
        TypeAnnotationDeclaration::Scalar(entity_name_wrapper) => {
            TCompilationProfile::TargetPlatform::format_server_field_scalar_type(
                db,
                entity_name_wrapper.0,
                indentation_level,
            )
        }
        TypeAnnotationDeclaration::Union(union_type_annotation) => {
            let mut s = String::new();
            let count = union_type_annotation.variants.len();
            for (index, variant) in union_type_annotation.variants.iter().enumerate() {
                let add_pipe = union_type_annotation.nullable || (index != count - 1);
                match variant {
                    UnionVariant::Scalar(entity_name_wrapper) => {
                        s.push_str(
                            &TCompilationProfile::TargetPlatform::format_server_field_scalar_type(
                                db,
                                entity_name_wrapper.0,
                                indentation_level,
                            ),
                        );
                    }
                    UnionVariant::Plural(p) => {
                        s.push_str("ReadonlyArray<");
                        s.push_str(&format_parameter_type(
                            db,
                            p.item.reference(),
                            indentation_level,
                        ));
                        s.push('>');
                    }
                }
                if add_pipe {
                    s.push_str(" | ");
                }
            }

            if union_type_annotation.nullable {
                s.push_str("null | void");
            }

            s
        }
        TypeAnnotationDeclaration::Plural(plural) => {
            let mut s = String::new();
            s.push_str("ReadonlyArray<");
            s.push_str(&format_parameter_type(
                db,
                plural.item.reference(),
                indentation_level,
            ));
            s.push('>');
            s
        }
    }
}
