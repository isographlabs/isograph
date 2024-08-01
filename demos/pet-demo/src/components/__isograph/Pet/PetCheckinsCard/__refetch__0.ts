import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
const queryText = 'mutation Pet__make_super ($checkin_id: ID!) {\
  make_checkin_super____checkin_id___v_checkin_id: make_checkin_super(checkin_id: $checkin_id) {\
    checkin {\
      ... on Checkin {\
        __typename,\
        id,\
        location,\
        time,\
      },\
    },\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "make_checkin_super",
    arguments: [
      [
        "checkin_id",
        { kind: "Variable", name: "checkin_id" },
      ],
    ],
    selections: [
      {
        kind: "Linked",
        fieldName: "checkin",
        arguments: null,
        selections: [
          {
            kind: "InlineFragment",
            type: "Checkin",
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
                fieldName: "location",
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "time",
                arguments: null,
              },
            ],
          },
        ],
      },
    ],
  },
];
const artifact: RefetchQueryNormalizationArtifact = {
  kind: "RefetchQuery",
  queryText,
  normalizationAst,
};

export default artifact;
