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
      documentId: "471a1acf574822f693b82f178322848f18ef3ab96053d5a1d7bdc39e89e277cf",
      operationName: "Query__make_super",
      operationKind: "Mutation",
      text: null,
    },
    normalizationAst,
  },
  concreteType: "Mutation",
};

export default artifact;
