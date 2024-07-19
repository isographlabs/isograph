import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst } from '@isograph/react';
const queryText = 'query Pet__refetch ($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Pet {\
      __typename,\
      id,\
      alt_tagline,\
      tagline,\
    },\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "node",
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    selections: [
      {
        kind: "InlineFragment",
        type: "Pet",
        selections: [
          {
            kind: "Scalar",
            fieldName: "__typename",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "alt_tagline",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "tagline",
            arguments: null,
          },
        ],
      },
    ],
  },
];
const artifact: any = {
  kind: "RefetchQuery",
  queryText,
  normalizationAst,
};

export default artifact;
