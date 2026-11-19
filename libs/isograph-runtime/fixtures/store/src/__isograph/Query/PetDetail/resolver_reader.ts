import type { ReaderArtifact } from "@isograph/runtime";
import type { Query__PetDetail__output_type } from "./output_type";
import type { Query__PetDetail__param } from "./param_type";
import { PetDetail as resolver } from "./../../../PetDetail";
const readerArtifact = (): ReaderArtifact<Query__PetDetail__param, Query__PetDetail__output_type, "Query"> =>({
    readerAst: [
      {
        kind: "Linked",
        alias: "pet",
        selectableName: "pet",
        arguments: [
          [
            "id",
            {
              kind: "Variable",
              name: "id"
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
            alias: "name",
            selectableName: "name",
            arguments: null
          },
          {
            kind: "Scalar",
            alias: "nickname",
            selectableName: "nickname",
            arguments: null
          },
          {
            kind: "Scalar",
            alias: "tagline",
            selectableName: "tagline",
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
                alias: "energy",
                selectableName: "energy",
                arguments: null
              }
            ]
          },
          {
            kind: "Linked",
            alias: "bestFriend",
            selectableName: "bestFriend",
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
                alias: "name",
                selectableName: "name",
                arguments: null
              },
              {
                kind: "Linked",
                alias: "bestFriend",
                selectableName: "bestFriend",
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
                    alias: "name",
                    selectableName: "name",
                    arguments: null
                  }
                ]
              }
            ]
          },
          {
            kind: "Linked",
            alias: "checkins",
            selectableName: "checkins",
            arguments: [
              [
                "limit",
                {
                  kind: "Literal",
                  value: 2
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
          },
          {
            kind: "Linked",
            alias: "pagedCheckins",
            selectableName: "checkins",
            arguments: [
              [
                "limit",
                {
                  kind: "Literal",
                  value: 2
                }
              ],
              [
                "skip",
                {
                  kind: "Variable",
                  name: "skip"
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
        ]
      }
    ],
    resolver: resolver
  });
export default readerArtifact;
