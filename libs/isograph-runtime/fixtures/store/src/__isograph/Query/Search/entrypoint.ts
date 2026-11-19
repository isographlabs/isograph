import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__Search__param } from "./param_type";
import type { Query__Search__output_type } from "./output_type";
import Query__Search__normalization_ast from "./normalization_ast";
import Query__Search__query_text from "./query_text";
import Query__Search__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__Search__param, Query__Search__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.Search",
  queryText: Query__Search__query_text,
  normalizationAst: Query__Search__normalization_ast,
  readerArtifact: Query__Search__resolver_reader
};
export default artifact;
