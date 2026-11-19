import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__SpeciesRoute__param } from "./param_type";
import type { Query__SpeciesRoute__output_type } from "./output_type";
import Query__SpeciesRoute__normalization_ast from "./normalization_ast";
import Query__SpeciesRoute__query_text from "./query_text";
import Query__SpeciesRoute__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__SpeciesRoute__param, Query__SpeciesRoute__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.SpeciesRoute",
  queryText: Query__SpeciesRoute__query_text,
  normalizationAst: Query__SpeciesRoute__normalization_ast,
  readerArtifact: ()=>Query__SpeciesRoute__resolver_reader()
};
export default artifact;
