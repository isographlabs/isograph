import type { ReaderArtifact } from "@isograph/runtime";
import type { Query__Leaves__output_type } from "./output_type";
import type { Query__Leaves__param } from "./param_type";
import { Leaves as resolver } from "./../../../Leaves";
const readerArtifact: ReaderArtifact<Query__Leaves__param, Query__Leaves__output_type, "Query"> = {
  readerAst: [
    {
      kind: "Scalar",
      alias: "greeting",
      selectableName: "greeting",
      arguments: null
    },
    {
      kind: "Scalar",
      alias: "literalGreeting",
      selectableName: "greeting",
      arguments: [
        [
          "loud",
          {
            kind: "Literal",
            value: true
          }
        ],
        [
          "name",
          {
            kind: "String",
            value: "Ada Lovelace"
          }
        ],
        [
          "times",
          {
            kind: "Literal",
            value: 3
          }
        ]
      ]
    },
    {
      kind: "Scalar",
      alias: "nullGreeting",
      selectableName: "greeting",
      arguments: [
        [
          "name",
          {
            kind: "Literal",
            value: null
          }
        ]
      ]
    },
    {
      kind: "Scalar",
      alias: "variableGreeting",
      selectableName: "greeting",
      arguments: [
        [
          "name",
          {
            kind: "Variable",
            name: "name"
          }
        ]
      ]
    },
    {
      kind: "Scalar",
      alias: "tags",
      selectableName: "tags",
      arguments: null
    },
    {
      kind: "Scalar",
      alias: "settings",
      selectableName: "settings",
      arguments: null
    }
  ],
  resolver: resolver
};
export default readerArtifact;
