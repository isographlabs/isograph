import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__PetDetailRoute__param } from "./param_type";
import type { Query__PetDetailRoute__output_type } from "./output_type";
import Query__PetDetailRoute__normalization_ast from "./normalization_ast";
import Query__PetDetailRoute__query_text from "./query_text";
import Query__PetDetailRoute__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__PetDetailRoute__param, Query__PetDetailRoute__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.PetDetailRoute",
  queryText: Query__PetDetailRoute__query_text,
  normalizationAst: Query__PetDetailRoute__normalization_ast,
  readerArtifact: Query__PetDetailRoute__resolver_reader
};
export default artifact;
