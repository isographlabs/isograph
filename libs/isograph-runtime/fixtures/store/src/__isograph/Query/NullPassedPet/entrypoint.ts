import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__NullPassedPet__param } from "./param_type";
import type { Query__NullPassedPet__output_type } from "./output_type";
import Query__NullPassedPet__normalization_ast from "./normalization_ast";
import Query__NullPassedPet__query_text from "./query_text";
import Query__NullPassedPet__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__NullPassedPet__param, Query__NullPassedPet__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.NullPassedPet",
  queryText: Query__NullPassedPet__query_text,
  normalizationAst: Query__NullPassedPet__normalization_ast,
  readerArtifact: ()=>Query__NullPassedPet__resolver_reader()
};
export default artifact;
