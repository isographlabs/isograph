import Mutation__SetTagline__normalization_ast from "./normalization_ast";
import Mutation__SetTagline__query_text from "./query_text";
import Mutation__SetTagline__resolver_reader from "./resolver_reader";
export default {
  kind: "Entrypoint",
  name: "Mutation.SetTagline",
  queryText: Mutation__SetTagline__query_text,
  normalizationAst: Mutation__SetTagline__normalization_ast,
  readerArtifact: Mutation__SetTagline__resolver_reader
};
