import type { ReaderArtifact } from "@isograph/runtime";
import type { Pet__PetStatsCard__output_type } from "./output_type";
import type { Pet__PetStatsCard__param } from "./param_type";
import { PetStatsCard as resolver } from "./../../../PetStatsCard";
const readerArtifact: ReaderArtifact<Pet__PetStatsCard__param, Pet__PetStatsCard__output_type, "Pet"> = {
  readerAst: [
    {
      kind: "Scalar",
      alias: "id",
      selectableName: "id",
      arguments: null
    },
    {
      kind: "Scalar",
      alias: "nickname",
      selectableName: "nickname",
      arguments: null
    },
    {
      kind: "Linked",
      alias: "stats",
      selectableName: "stats",
      arguments: null,
      selections: [
        {
          kind: "Scalar",
          alias: "weight",
          selectableName: "weight",
          arguments: null
        },
        {
          kind: "Scalar",
          alias: "intelligence",
          selectableName: "intelligence",
          arguments: null
        },
        {
          kind: "Scalar",
          alias: "cuteness",
          selectableName: "cuteness",
          arguments: null
        },
        {
          kind: "Scalar",
          alias: "hunger",
          selectableName: "hunger",
          arguments: null
        },
        {
          kind: "Scalar",
          alias: "sociability",
          selectableName: "sociability",
          arguments: null
        },
        {
          kind: "Scalar",
          alias: "energy",
          selectableName: "energy",
          arguments: null
        }
      ]
    }
  ],
  resolver: resolver
};
export default readerArtifact;
