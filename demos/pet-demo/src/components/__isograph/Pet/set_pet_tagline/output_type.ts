import type {ExtractSecondParam} from '@isograph/react';
const includeReadOutData = (variables: any, readOutData: any) => {
  variables.input = variables.input ?? {};
  variables.input.id = readOutData.id;
  return variables;
};

import { makeNetworkRequest, type IsographEnvironment, type IsographEntrypoint } from '@isograph/react';
const resolver = (
  environment: IsographEnvironment,
  artifact: IsographEntrypoint<any, any>,
  readOutData: any,
  filteredVariables: any
) => (mutationParams: any) => {
  const variables = includeReadOutData({...filteredVariables, ...mutationParams}, readOutData);
  makeNetworkRequest(environment, artifact, variables);
};

// the type, when read out (either via useLazyReference or via graph)
export type Pet__set_pet_tagline__outputType = (params: any) => void;
