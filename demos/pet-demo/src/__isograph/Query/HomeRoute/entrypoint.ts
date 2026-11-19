import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__HomeRoute__param } from "./param_type";
import type { Query__HomeRoute__output_type } from "./output_type";
import Query__HomeRoute__normalization_ast from "./normalization_ast";
import Query__HomeRoute__query_text from "./query_text";
import Query__HomeRoute__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__HomeRoute__param, Query__HomeRoute__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.HomeRoute",
  queryText: Query__HomeRoute__query_text,
  normalizationAst: Query__HomeRoute__normalization_ast,
  readerArtifact: Query__HomeRoute__resolver_reader
};
export default artifact;
