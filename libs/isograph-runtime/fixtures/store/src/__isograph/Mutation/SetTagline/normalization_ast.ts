import type { NormalizationAst } from "@isograph/runtime";
const normalizationAst: NormalizationAst<"Mutation"> = {
  discriminatorResponseKey: "__typename",
  root: {
    entityName: "Mutation",
    id: "__ROOT"
  },
  selections: [
    {
      kind: "Selection",
      responseKey: "setTagline____input___o_id__v_id_tagline__v_tagline_c",
      selectableName: "setTagline",
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
      target: {
        entity: {
          kind: "Known",
          entityName: "SetTaglinePayload"
        },
        identity: {
          kind: "Omit"
        },
        selections: [
          {
            kind: "Selection",
            responseKey: null,
            selectableName: "pet",
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
                  selectableName: "tagline",
                  arguments: null,
                  target: null
                }
              ]
            }
          },
          {
            kind: "Selection",
            responseKey: null,
            selectableName: "query",
            arguments: null,
            target: {
              entity: {
                kind: "Known",
                entityName: "Query"
              },
              identity: {
                kind: "Fixed",
                id: "__ROOT"
              },
              selections: [
                {
                  kind: "Selection",
                  responseKey: null,
                  selectableName: "greeting",
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
