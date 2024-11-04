import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__meNameSuccessor__param} from './param_type';
import {Query__meNameSuccessor__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query meNameSuccessor  {\
  me {\
    id,\
    name,\
    successor {\
      id,\
      successor {\
        id,\
        name,\
      },\
    },\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "me",
    arguments: null,
    concreteType: "Economist",
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
      {
        kind: "Linked",
        fieldName: "successor",
        arguments: null,
        concreteType: "Economist",
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "successor",
            arguments: null,
            concreteType: "Economist",
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
        ],
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  Query__meNameSuccessor__param,
  Query__meNameSuccessor__output_type
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
