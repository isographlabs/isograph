import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "pet",
      arguments: [
        [
          "id",
          { kind: "Variable", name: "id" },
        ],
      ],
      concreteType: "Pet",
      selections: [
        {
          kind: "Linked",
          fieldName: "stats",
          arguments: null,
          concreteType: "PetStats",
          selections: [
            {
              kind: "Scalar",
              fieldName: "cuteness",
              arguments: null,
            },
            {
              kind: "Scalar",
              fieldName: "energy",
              arguments: null,
            },
            {
              kind: "Scalar",
              fieldName: "hunger",
              arguments: null,
            },
            {
              kind: "Scalar",
              fieldName: "intelligence",
              arguments: null,
            },
            {
              kind: "Scalar",
              fieldName: "sociability",
              arguments: null,
            },
            {
              kind: "Scalar",
              fieldName: "weight",
              arguments: null,
            },
          ],
        },
      ],
    },
  ],
};
const artifact: RefetchQueryNormalizationArtifact = {
  kind: "RefetchQuery",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      documentId: "b18b7c68180b18b1f81441f76283887db14d010900ac171db2d7d78dd2c4ab0f",
      operationName: "Query__refetch_pet_stats",
      operationKind: "Query",
      text: null,
    },
    normalizationAst,
  },
  concreteType: "Query",
};

export default artifact;
