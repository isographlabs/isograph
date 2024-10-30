import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {User__asUser__param} from './param_type';
import {User__asUser__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query asUser ($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on User {\
      __typename,\
      id,\
      bio,\
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
    concreteType: null,
    selections: [
      {
        kind: "InlineFragment",
        type: "User",
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
            fieldName: "bio",
            arguments: null,
          },
        ],
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  User__asUser__param,
  User__asUser__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
