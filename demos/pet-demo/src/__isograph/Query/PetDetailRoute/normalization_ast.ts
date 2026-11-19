import type { NormalizationAst } from "@isograph/runtime";
const normalizationAst: NormalizationAst<"Query"> = {
  discriminatorResponseKey: "__typename",
  root: {
    entityName: "Query",
    id: "__ROOT"
  },
  selections: [
    {
      kind: "Selection",
      responseKey: "pet____id___v_id",
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
      target: {
        entity: {
          kind: "Known",
          entityName: "Pet"
        },
        identity: {
          kind: "Select",
          selectableName: "id"
        },
        selections: [
          {
            kind: "Selection",
            responseKey: null,
            selectableName: "id",
            arguments: null,
            target: null
          },
          {
            kind: "Selection",
            responseKey: null,
            selectableName: "best_friend_relationship",
            arguments: null,
            target: {
              entity: {
                kind: "Known",
                entityName: "BestFriendRelationship"
              },
              identity: {
                kind: "Omit"
              },
              selections: [
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "best_friend",
                  arguments: null,
                  target: {
                    entity: {
                      kind: "Known",
                      entityName: "Pet"
                    },
                    identity: {
                      kind: "Select",
                      selectableName: "id"
                    },
                    selections: [
                      {
                        kind: "Selection",
                        responseKey: null,
                        selectableName: "id",
                        arguments: null,
                        target: null
                      },
                      {
                        kind: "Selection",
                        responseKey: null,
                        selectableName: "firstName",
                        arguments: null,
                        target: null
                      },
                      {
                        kind: "Selection",
                        responseKey: null,
                        selectableName: "lastName",
                        arguments: null,
                        target: null
                      }
                    ]
                  }
                },
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "picture_together",
                  arguments: null,
                  target: null
                }
              ]
            }
          },
          {
            kind: "Selection",
            responseKey: "checkins____limit___l_5____skip___l_0",
            selectableName: "checkins",
            arguments: [
              [
                "limit",
                {
                  kind: "Literal",
                  value: 5
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
            target: {
              entity: {
                kind: "Known",
                entityName: "Checkin"
              },
              identity: {
                kind: "Select",
                selectableName: "id"
              },
              selections: [
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "id",
                  arguments: null,
                  target: null
                },
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "location",
                  arguments: null,
                  target: null
                },
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "time",
                  arguments: null,
                  target: null
                }
              ]
            }
          },
          {
            kind: "Selection",
            responseKey: null,
            selectableName: "firstName",
            arguments: null,
            target: null
          },
          {
            kind: "Selection",
            responseKey: null,
            selectableName: "lastName",
            arguments: null,
            target: null
          },
          {
            kind: "Selection",
            responseKey: null,
            selectableName: "nickname",
            arguments: null,
            target: null
          },
          {
            kind: "Selection",
            responseKey: null,
            selectableName: "stats",
            arguments: null,
            target: {
              entity: {
                kind: "Known",
                entityName: "PetStats"
              },
              identity: {
                kind: "Omit"
              },
              selections: [
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "cuteness",
                  arguments: null,
                  target: null
                },
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "energy",
                  arguments: null,
                  target: null
                },
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "hunger",
                  arguments: null,
                  target: null
                },
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "intelligence",
                  arguments: null,
                  target: null
                },
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "sociability",
                  arguments: null,
                  target: null
                },
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "weight",
                  arguments: null,
                  target: null
                }
              ]
            }
          }
        ]
      }
    }
  ]
};
export default normalizationAst;
