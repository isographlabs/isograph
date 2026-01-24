import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
import queryText from './__refetch__query_text__5';

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: true,
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
          isFallible: true,
          fieldName: "stats",
          arguments: null,
          concreteType: "PetStats",
          selections: [
            {
              kind: "Scalar",
              isFallible: true,
              fieldName: "cuteness",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: true,
              fieldName: "energy",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: true,
              fieldName: "hunger",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: true,
              fieldName: "intelligence",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: true,
              fieldName: "sociability",
              arguments: null,
            },
            {
              kind: "Scalar",
              isFallible: true,
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
      text: queryText,
    },
    normalizationAst,
  },
  concreteType: "Query",
};

export default artifact;
