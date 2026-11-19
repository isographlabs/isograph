import type { ReaderArtifact } from "@isograph/runtime";
import type { Pet__RecentCheckins__output_type } from "./output_type";
import type { Pet__RecentCheckins__param } from "./param_type";
import { RecentCheckins as resolver } from "./../../../RecentCheckins";
const readerArtifact = (): ReaderArtifact<Pet__RecentCheckins__param, Pet__RecentCheckins__output_type, "Pet"> =>({
    readerAst: [
      {
        kind: "Linked",
        alias: "checkins",
        selectableName: "checkins",
        arguments: [
          [
            "limit",
            {
              kind: "Variable",
              name: "n"
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
          }
        ]
      }
    ],
    resolver: resolver
  });
export default readerArtifact;
