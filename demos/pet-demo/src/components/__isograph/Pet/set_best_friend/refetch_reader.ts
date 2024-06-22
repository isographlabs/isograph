import type {RefetchReaderArtifact, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
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
) => (mutationParams: any) => {
  const variables = includeReadOutData({...filteredVariables, ...mutationParams}, readOutData);
  makeNetworkRequest(environment, artifact, variables);
};


const readerAst: ReaderAst<unknown> = [
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
