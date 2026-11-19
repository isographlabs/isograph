import type { ReaderArtifact } from "@isograph/runtime";
import type { Query__HomeRoute__output_type } from "./output_type";
import type { Query__HomeRoute__param } from "./param_type";
import { HomeRoute as resolver } from "./../../../HomeRoute";
import Pet__PetSummaryCard__resolver_reader from "./../../Pet/PetSummaryCard/resolver_reader";
const readerArtifact: ReaderArtifact<Query__HomeRoute__param, Query__HomeRoute__output_type, "Query"> = {
  readerAst: [
    {
      kind: "Linked",
      alias: "pets",
      selectableName: "pets",
      arguments: null,
      selections: [
        {
          kind: "Scalar",
          alias: "id",
          selectableName: "id",
          arguments: null
        },
        {
          kind: "Resolver",
          alias: "PetSummaryCard",
          arguments: null,
          readerArtifact: Pet__PetSummaryCard__resolver_reader
        }
      ]
    }
  ],
  resolver: resolver
};
export default readerArtifact;
