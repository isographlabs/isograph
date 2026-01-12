mod impl_base_types;

extern crate proc_macro2;

use impl_base_types::BaseType;
use proc_macro::TokenStream;

static SELECTION_TYPE_BASE_TYPE: BaseType = BaseType {
    variant_names: &["Scalar", "Object"],
    name: "SelectionType",
    crate_name: "isograph_lang_types",
};

static DEFINITION_LOCATION_BASE_TYPE: BaseType = BaseType {
    variant_names: &["Server", "Client"],
    name: "DefinitionLocation",
    crate_name: "isograph_lang_types",
};

#[proc_macro_attribute]
pub fn impl_for_selection_type(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_base_types::impl_base_types(
        args,
        input,
        &[SELECTION_TYPE_BASE_TYPE],
        "impl_for_selection_types",
    )
}

#[proc_macro_attribute]
pub fn impl_for_definition_location(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_base_types::impl_base_types(
        args,
        input,
        &[DEFINITION_LOCATION_BASE_TYPE],
        "impl_for_definition_location",
    )
}

#[proc_macro_attribute]
pub fn impl_for_all_base_types(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_base_types::impl_base_types(
        args,
        input,
        &[SELECTION_TYPE_BASE_TYPE, DEFINITION_LOCATION_BASE_TYPE],
        "impl_for_all_base_types",
    )
}
