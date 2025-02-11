import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__HomeRoute__param} from './param_type';
import {Query__HomeRoute__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query HomeRoute  {\
  pets {\
    id,\
    name,\
    picture,\
    tagline,\
  },\
}';

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "pets",
      arguments: null,
      concreteType: "Pet",
      selections: [
        {
          kind: "Scalar",
          fieldName: "id",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "name",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "picture",
          arguments: null,
          isUpdatable: false,
        },
        {
          kind: "Scalar",
          fieldName: "tagline",
          arguments: null,
          isUpdatable: false,
        },
      ],
    },
  ],
};
const artifact: IsographEntrypoint<
  Query__HomeRoute__param,
  Query__HomeRoute__output_type,
  NormalizationAst
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
