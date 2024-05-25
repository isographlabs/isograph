import type {RefetchReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { User____refetch__param } from './param_type';
const includeReadOutData = (variables: any, readOutData: any) => {
  variables.id = readOutData.id;
  return variables;
};

import { makeNetworkRequest, type IsographEnvironment } from '@isograph/react';
const resolver = (
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact,
  readOutData: any,
  filteredVariables: any
) => () => {
  const variables = includeReadOutData(filteredVariables, readOutData);
  return makeNetworkRequest(environment, artifact, variables);
};


const readerAst: ReaderAst<User____refetch__param> = [
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
