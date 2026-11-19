import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__Feed__param } from "./param_type";
import type { Query__Feed__output_type } from "./output_type";
import Query__Feed__normalization_ast from "./normalization_ast";
import Query__Feed__query_text from "./query_text";
import Query__Feed__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__Feed__param, Query__Feed__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.Feed",
  queryText: Query__Feed__query_text,
  normalizationAst: Query__Feed__normalization_ast,
  readerArtifact: Query__Feed__resolver_reader
};
export default artifact;
