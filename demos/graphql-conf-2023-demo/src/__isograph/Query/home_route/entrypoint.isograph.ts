import type {IsographFetchableResolver, FragmentReference, NormalizationAst, RefetchQueryArtifactWrapper} from '@isograph/react';
import type {ReadFromStoreType, ResolverParameterType, ReadOutType} from './reader.isograph';
import readerResolver from './reader.isograph';
const nestedRefetchQueries: RefetchQueryArtifactWrapper[] = [];

const queryText = 'query home_route  {\
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
const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "FetchableResolver",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
