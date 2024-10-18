import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__UserPage__param} from './param_type';
import {Query__UserPage__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query UserPage ($userLogin: String!) {\
  user____login___v_userLogin: user(login: $userLogin) {\
    id,\
    name,\
  },\
  viewer {\
    id,\
    avatarUrl,\
    name,\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "user",
    arguments: [
      [
        "login",
        { kind: "Variable", name: "userLogin" },
      ],
    ],
    concreteType: "User",
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        arguments: null,
      },
    ],
  },
  {
    kind: "Linked",
    fieldName: "viewer",
    arguments: null,
    concreteType: "User",
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "avatarUrl",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        arguments: null,
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  Query__UserPage__param,
  Query__UserPage__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  concreteType: "Query",
  queryType: "Query",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
