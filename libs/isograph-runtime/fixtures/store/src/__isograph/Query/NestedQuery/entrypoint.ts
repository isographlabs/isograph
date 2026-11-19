import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__NestedQuery__param } from "./param_type";
import type { Query__NestedQuery__output_type } from "./output_type";
import Query__NestedQuery__normalization_ast from "./normalization_ast";
import Query__NestedQuery__query_text from "./query_text";
import Query__NestedQuery__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__NestedQuery__param, Query__NestedQuery__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.NestedQuery",
  queryText: Query__NestedQuery__query_text,
  normalizationAst: Query__NestedQuery__normalization_ast,
  readerArtifact: ()=>Query__NestedQuery__resolver_reader()
};
export default artifact;
