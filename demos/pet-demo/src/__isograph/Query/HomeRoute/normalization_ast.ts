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
      responseKey: null,
      selectableName: "pets",
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
            selectableName: "age",
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
            selectableName: "picture",
            arguments: null,
            target: null
          },
          {
            kind: "Selection",
            responseKey: null,
            selectableName: "species",
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
    }
  ]
};
export default normalizationAst;
