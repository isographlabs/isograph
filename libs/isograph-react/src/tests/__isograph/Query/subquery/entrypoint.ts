import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__subquery__param} from './param_type';
import {Query__subquery__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query subquery ($id: ID!) {\
  query {\
    node____id___v_id: node(id: $id) {\
      __typename,\
      id,\
    },\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "query",
    arguments: null,
    concreteType: "Query",
    selections: [
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
            kind: "Scalar",
            fieldName: "__typename",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
        ],
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  Query__subquery__param,
  Query__subquery__output_type
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    queryText,
    normalizationAst,
  },
  concreteType: "Query",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
