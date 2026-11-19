import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__PetSummaries__param } from "./param_type";
import type { Query__PetSummaries__output_type } from "./output_type";
import Query__PetSummaries__normalization_ast from "./normalization_ast";
import Query__PetSummaries__query_text from "./query_text";
import Query__PetSummaries__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__PetSummaries__param, Query__PetSummaries__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.PetSummaries",
  queryText: Query__PetSummaries__query_text,
  normalizationAst: Query__PetSummaries__normalization_ast,
  readerArtifact: ()=>Query__PetSummaries__resolver_reader()
};
export default artifact;
