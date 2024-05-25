import { RefetchQueryNormalizationArtifact } from '@isograph/react';
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

export type User____refetch__outputType = () => void;