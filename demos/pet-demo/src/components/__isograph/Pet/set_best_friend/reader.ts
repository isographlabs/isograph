import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet__set_best_friend__param } from './param_type.ts';
import { Pet__set_best_friend__outputType } from './output_type.ts';
const includeReadOutData = (variables: any, readOutData: any) => {
  variables.id = readOutData.id;
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


const readerAst: ReaderAst<Pet__set_best_friend__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

const artifact: ReaderArtifact<
  Pet__set_best_friend__param,
  Pet__set_best_friend__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
