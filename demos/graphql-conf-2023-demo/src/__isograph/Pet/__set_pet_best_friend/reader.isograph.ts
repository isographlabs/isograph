import type {ReaderArtifact, ReaderAst} from '@isograph/react';
const includeReadOutData = (variables, readOutData) => {
  variables.id = readOutData.id;
  return variables;
};

import { makeNetworkRequest } from '@isograph/react';
const resolver = (artifact, readOutData, filteredVariables) => (mutationParams) => {
  const variables = includeReadOutData({...filteredVariables, ...mutationParams}, readOutData);
  makeNetworkRequest(artifact, variables);
};


// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = any;

export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

export type ResolverParameterType = {
  id: string,
};

// The type, when returned from the resolver
export type ResolverReturnType = any;

const artifact: ReaderArtifact<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
