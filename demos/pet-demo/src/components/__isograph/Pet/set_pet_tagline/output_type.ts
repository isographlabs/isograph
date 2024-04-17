import { RefetchQueryNormalizationArtifact } from '@isograph/react';
const includeReadOutData = (variables: any, readOutData: any) => {
  variables.input = variables.input ?? {};
  variables.input.id = readOutData.id;
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

export type Pet__set_pet_tagline__outputType = (params: any) => void;