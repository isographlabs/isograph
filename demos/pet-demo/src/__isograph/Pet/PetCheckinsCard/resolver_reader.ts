import type { ReaderArtifact } from "@isograph/runtime";
import type { Pet__PetCheckinsCard__output_type } from "./output_type";
import type { Pet__PetCheckinsCard__param } from "./param_type";
import { PetCheckinsCard as resolver } from "./../../../PetCheckinsCard";
const readerArtifact = (): ReaderArtifact<Pet__PetCheckinsCard__param, Pet__PetCheckinsCard__output_type, "Pet"> =>({
    readerAst: [
      {
        kind: "Scalar",
        alias: "id",
        selectableName: "id",
        arguments: null
      },
      {
        kind: "Linked",
        alias: "checkins",
        selectableName: "checkins",
        arguments: [
          [
            "limit",
            {
              kind: "Variable",
              name: "limit"
            }
          ],
          [
            "skip",
            {
              kind: "Literal",
              value: 0
            }
          ]
        ],
        selections: [
          {
            kind: "Scalar",
            alias: "id",
            selectableName: "id",
            arguments: null
          },
          {
            kind: "Scalar",
            alias: "location",
            selectableName: "location",
            arguments: null
          },
          {
            kind: "Scalar",
            alias: "time",
            selectableName: "time",
            arguments: null
          }
        ]
      }
    ],
    resolver: resolver
  });
export default readerArtifact;
