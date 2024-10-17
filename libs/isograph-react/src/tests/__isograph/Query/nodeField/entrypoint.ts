import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__nodeField__param} from './param_type';
import {Query__nodeField__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query nodeField ($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    __typename,\
    id,\
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
];
const artifact: IsographEntrypoint<
  Query__nodeField__param,
  Query__nodeField__output_type
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
