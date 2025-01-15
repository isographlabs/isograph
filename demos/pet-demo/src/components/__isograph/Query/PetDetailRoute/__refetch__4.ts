import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
const queryText = 'query Query__refetch_pet_stats ($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    stats {\
      cuteness,\
      energy,\
      hunger,\
      intelligence,\
      sociability,\
      weight,\
    },\
  },\
}';

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
    queryText,
    normalizationAst,
  },
  concreteType: "Query",
};

export default artifact;
