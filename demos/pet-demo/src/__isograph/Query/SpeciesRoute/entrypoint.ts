import Query__SpeciesRoute__normalization_ast from "./normalization_ast";
import Query__SpeciesRoute__query_text from "./query_text";
import Query__SpeciesRoute__resolver_reader from "./resolver_reader";
export default {
  kind: "Entrypoint",
  name: "Query.SpeciesRoute",
  queryText: Query__SpeciesRoute__query_text,
  normalizationAst: Query__SpeciesRoute__normalization_ast,
  readerArtifact: Query__SpeciesRoute__resolver_reader
};
