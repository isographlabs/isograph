import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__PetDetail__param } from "./param_type";
import type { Query__PetDetail__output_type } from "./output_type";
import Query__PetDetail__normalization_ast from "./normalization_ast";
import Query__PetDetail__query_text from "./query_text";
import Query__PetDetail__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__PetDetail__param, Query__PetDetail__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.PetDetail",
  queryText: Query__PetDetail__query_text,
  normalizationAst: Query__PetDetail__normalization_ast,
  readerArtifact: Query__PetDetail__resolver_reader
};
export default artifact;
