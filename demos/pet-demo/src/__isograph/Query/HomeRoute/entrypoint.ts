import Query__HomeRoute__normalization_ast from "./normalization_ast";
import Query__HomeRoute__query_text from "./query_text";
import Query__HomeRoute__resolver_reader from "./resolver_reader";
export default {
  kind: "Entrypoint",
  name: "Query.HomeRoute",
  queryText: Query__HomeRoute__query_text,
  normalizationAst: Query__HomeRoute__normalization_ast,
  readerArtifact: Query__HomeRoute__resolver_reader
};
