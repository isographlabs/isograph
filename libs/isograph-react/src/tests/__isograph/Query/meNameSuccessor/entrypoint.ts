import type {IsographEntrypoint, NormalizationAst, RefetchQueryArtifactWrapper} from '@isograph/react';
import {Query__meNameSuccessor__param} from './param_type';
import {Query__meNameSuccessor__outputType} from './output_type';
import readerResolver from './reader';
const nestedRefetchQueries: RefetchQueryArtifactWrapper[] = [];

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
  Query__meNameSuccessor__outputType
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
