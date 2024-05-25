import type {RefetchReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Pet____refetch__param } from './param_type';
import { makeNetworkRequest, type IsographEnvironment } from '@isograph/react';
const resolver = (
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact,
  variables: any
) => () => makeNetworkRequest(environment, artifact, variables);

const readerAst: ReaderAst<Pet____refetch__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

const artifact: RefetchReaderArtifact = {
  kind: "RefetchReaderArtifact",
  // @ts-ignore
  resolver,
  readerAst,
};

export default artifact;
