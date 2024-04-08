import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
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


// the type, when read out (either via useLazyReference or via graph)
export type Pet__set_best_friend__outputType = (params: any) => void;

const readerAst: ReaderAst<Pet__set_best_friend__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

export type Pet__set_best_friend__param = {
  id: string,
};

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
