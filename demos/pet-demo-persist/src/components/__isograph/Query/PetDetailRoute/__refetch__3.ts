import type { IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst, RefetchQueryNormalizationArtifact } from '@isograph/react';
const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
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
          fieldName: "checkin",
          arguments: null,
          concreteType: null,
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
  ],
};
const artifact: RefetchQueryNormalizationArtifact = {
  kind: "RefetchQuery",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    operation: {
      kind: "Operation",
      documentId: "822d6bd2feab25f803b3401939e779ae4110b4a23fcb5d1a49e05d64c701fbcf",
      operationName: "Query__make_super",
      operationKind: "Mutation",
      text: null,
    },
    normalizationAst,
  },
  concreteType: "Mutation",
};

export default artifact;
