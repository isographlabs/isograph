import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst} from '@isograph/react';
const queryText = 'mutation UserStatus__update_user_bio ($first: Int!, $id: ID!, $bio: String) {\
update_user_bio____bio___$bio: update_user_bio(bio: $bio) {\
user { \
  id,\
  emoji,\
  user {\
    id,\
    repositories____last___first: repositories(last: $first) {\
      __typename,\
    },\
  },\
}}}';

const normalizationAst: NormalizationAst = [{
  kind: "Linked",
  fieldName: "update_user_bio",
  arguments: [
    {
      argumentName: "bio",
      variableName: "bio",
    },
  ],
  selections: [
    {
      kind: "Linked",
      fieldName: "user",
      arguments: null,
      selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "emoji",
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "user",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "repositories",
            arguments: [
              {
                argumentName: "last",
                variableName: "first",
              },
            ],
            selections: [
              {
                kind: "Scalar",
                fieldName: "__typename",
                arguments: null,
              },
            ],
          },
        ],
      },
    ],
    },
  ],
}];
const artifact: any = {
  kind: "RefetchQuery",
  queryText,
  normalizationAst,
};

export default artifact;
