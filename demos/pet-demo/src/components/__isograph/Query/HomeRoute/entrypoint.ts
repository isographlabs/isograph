import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__HomeRoute__param} from './param_type';
import {Query__HomeRoute__outputType} from './output_type';
import readerResolver from './reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query HomeRoute  {\
  pets {\
    id,\
    name,\
    picture,\
    tagline,\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "pets",
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
        kind: "Scalar",
        fieldName: "picture",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "tagline",
        arguments: null,
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  Query__HomeRoute__param,
  Query__HomeRoute__outputType
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
