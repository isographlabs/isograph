import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
import queryText from './__refetch__query_text__0';

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      isFallible: false,
      fieldName: "make_checkin_super",
      arguments: [
        [
          "checkin_id",
          { kind: "Variable", name: "checkin_id" },
        ],
      ],
      concreteType: "MakeCheckinSuperResponse",
      selections: [
        {
          kind: "Linked",
          isFallible: true,
          fieldName: "icheckin",
          arguments: null,
          concreteType: "Checkin",
          selections: [
            {
              kind: "InlineFragment",
              type: "Checkin",
              selections: [
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "__typename",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "id",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "location",
                  arguments: null,
                },
                {
                  kind: "Scalar",
                  isFallible: false,
                  fieldName: "time",
                  arguments: null,
                },
              ],
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
  concreteType: "Mutation",
};

export default artifact;
