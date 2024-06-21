import { RefetchQueryNormalizationArtifact } from '@isograph/react';
const includeReadOutData = (variables: any, readOutData: any) => {
  variables.checkin_id = readOutData.id;
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

export type ICheckin__make_super__output_type = (params: any) => void;