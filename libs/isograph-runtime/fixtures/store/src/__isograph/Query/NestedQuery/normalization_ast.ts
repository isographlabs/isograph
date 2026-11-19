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
      selectableName: "greeting",
      arguments: null,
      target: null
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
            selectableName: "tags",
            arguments: null,
            target: null
          }
        ]
      }
    }
  ]
};
export default normalizationAst;
