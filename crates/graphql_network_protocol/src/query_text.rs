use common_lang_types::ArtifactFileName;
use intern::string_key::Intern;
use isograph_compiler::{EntrypointOperation, ProtocolQueryText};
use prelude::Postfix;

use crate::GraphQLNetworkProtocol;
use crate::print_operation::print_operation;

pub const QUERY_TEXT_FILE_NAME: &str = "query_text.graphql";

pub(crate) fn query_text(
    operation: &EntrypointOperation<'_, GraphQLNetworkProtocol>,
) -> ProtocolQueryText {
    ProtocolQueryText {
        text: print_operation(operation),
        file_name: QUERY_TEXT_FILE_NAME
            .intern()
            .to::<ArtifactFileName>()
            .wrap_some(),
    }
}
