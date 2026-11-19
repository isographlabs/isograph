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
      responseKey: "node____id___v_id",
      selectableName: "node",
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
          kind: "Discriminator"
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
          }
        ]
      }
    },
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
            selectableName: "bestFriend",
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
                }
              ]
            }
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
              selections: []
            }
          }
        ]
      }
    }
  ]
};
export default normalizationAst;
