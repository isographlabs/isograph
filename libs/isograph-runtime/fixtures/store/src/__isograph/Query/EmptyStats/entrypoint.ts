import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__EmptyStats__param } from "./param_type";
import type { Query__EmptyStats__output_type } from "./output_type";
import Query__EmptyStats__normalization_ast from "./normalization_ast";
import Query__EmptyStats__query_text from "./query_text";
import Query__EmptyStats__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__EmptyStats__param, Query__EmptyStats__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.EmptyStats",
  queryText: Query__EmptyStats__query_text,
  normalizationAst: Query__EmptyStats__normalization_ast,
  readerArtifact: ()=>Query__EmptyStats__resolver_reader()
};
export default artifact;
