import type { ReaderArtifact } from "@isograph/runtime";
import type { Pet__PetSummaryCard__output_type } from "./output_type";
import type { Pet__PetSummaryCard__param } from "./param_type";
import { PetSummaryCard as resolver } from "./../../../PetSummaryCard";
import Pet__fullName__resolver_reader from "./../fullName/resolver_reader";
const readerArtifact: ReaderArtifact<Pet__PetSummaryCard__param, Pet__PetSummaryCard__output_type, "Pet"> = {
  readerAst: [
    {
      kind: "Scalar",
      alias: "id",
      selectableName: "id",
      arguments: null
    },
    {
      kind: "Resolver",
      alias: "fullName",
      arguments: null,
      readerArtifact: Pet__fullName__resolver_reader
    },
    {
      kind: "Scalar",
      alias: "nickname",
      selectableName: "nickname",
      arguments: null
    },
    {
      kind: "Scalar",
      alias: "age",
      selectableName: "age",
      arguments: null
    },
    {
      kind: "Scalar",
      alias: "picture",
      selectableName: "picture",
      arguments: null
    },
    {
      kind: "Scalar",
      alias: "tagline",
      selectableName: "tagline",
      arguments: null
    },
    {
      kind: "Scalar",
      alias: "species",
      selectableName: "species",
      arguments: null
    }
  ],
  resolver: resolver
};
export default readerArtifact;
