import type { ReaderArtifact } from "@isograph/runtime";
import type { Mutation__SetTagline__output_type } from "./output_type";
import type { Mutation__SetTagline__param } from "./param_type";
import { SetTagline as resolver } from "./../../../SetTagline";
const readerArtifact: ReaderArtifact<Mutation__SetTagline__param, Mutation__SetTagline__output_type, "Mutation"> = {
  readerAst: [
    {
      kind: "Linked",
      alias: "set_pet_tagline",
      selectableName: "set_pet_tagline",
      arguments: [
        [
          "input",
          {
            kind: "Object",
            value: [
              [
                "id",
                {
                  kind: "Variable",
                  name: "id"
                }
              ],
              [
                "tagline",
                {
                  kind: "Variable",
                  name: "tagline"
                }
              ]
            ]
          }
        ]
      ],
      selections: [
        {
          kind: "Linked",
          alias: "pet",
          selectableName: "pet",
          arguments: null,
          selections: [
            {
              kind: "Scalar",
              alias: "id",
              selectableName: "id",
              arguments: null
            },
            {
              kind: "Scalar",
              alias: "tagline",
              selectableName: "tagline",
              arguments: null
            }
          ]
        }
      ]
    }
  ],
  resolver: resolver
};
export default readerArtifact;
