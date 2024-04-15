import type {IsographEntrypoint, NormalizationAst, RefetchQueryArtifactWrapper} from '@isograph/react';
import {Query__meName__param} from './param_type';
import {Query__meName__outputType} from './output_type';
import readerResolver from './reader';
const nestedRefetchQueries: RefetchQueryArtifactWrapper[] = [];

const queryText = 'query meName  {\
  me {\
    id,\
    name,\
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
    ],
  },
];
const artifact: IsographEntrypoint<
  Query__meName__param,
  Query__meName__outputType
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
