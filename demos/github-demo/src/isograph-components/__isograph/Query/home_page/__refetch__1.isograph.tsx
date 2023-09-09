import type {IsographFetchableResolver, ReaderAst, FragmentReference, NormalizationAst} from '@isograph/react';
const queryText = 'mutation UserStatus__update_user_bio ($id: ID!, $bio: String) { __update_user_bio(bio: $bio) { user { \
  id,\
  emoji,\
  user {\
    id,\
  },\
}}}';

const normalizationAst: NormalizationAst = [{ kind: "Linked", fieldName: "node", alias: null, arguments: [{ argumentName: "id", variableName: "id" }], selections: [
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
    ],
  },
] }];
const artifact: any = {
  kind: "RefetchQuery",
  queryText,
  normalizationAst,
};

export default artifact;
