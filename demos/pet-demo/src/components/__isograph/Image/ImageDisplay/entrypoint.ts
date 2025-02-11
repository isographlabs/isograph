import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Image__ImageDisplay__param} from './param_type';
import {Image__ImageDisplay__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query ImageDisplay ($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Image {\
      __typename,\
      id,\
      url,\
    },\
  },\
}';

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
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
          kind: "InlineFragment",
          type: "Image",
          selections: [
            {
              kind: "Scalar",
              fieldName: "__typename",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "id",
              arguments: null,
              isUpdatable: false,
            },
            {
              kind: "Scalar",
              fieldName: "url",
              arguments: null,
              isUpdatable: false,
            },
          ],
        },
      ],
    },
  ],
};
const artifact: IsographEntrypoint<
  Image__ImageDisplay__param,
  Image__ImageDisplay__output_type,
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
