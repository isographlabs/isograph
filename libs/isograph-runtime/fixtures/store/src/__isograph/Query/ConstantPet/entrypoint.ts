import type { IsographEntrypoint } from "@isograph/runtime";
import type { Query__ConstantPet__param } from "./param_type";
import type { Query__ConstantPet__output_type } from "./output_type";
import Query__ConstantPet__normalization_ast from "./normalization_ast";
import Query__ConstantPet__query_text from "./query_text";
import Query__ConstantPet__resolver_reader from "./resolver_reader";
const artifact: IsographEntrypoint<Query__ConstantPet__param, Query__ConstantPet__output_type, "Query"> = {
  kind: "Entrypoint",
  name: "Query.ConstantPet",
  queryText: Query__ConstantPet__query_text,
  normalizationAst: Query__ConstantPet__normalization_ast,
  readerArtifact: ()=>Query__ConstantPet__resolver_reader()
};
export default artifact;
