import type {ReaderArtifact, ReaderAst} from '@isograph/react';
const includeReadOutData = (variables, readOutData) => {
  variables.input = variables.input ?? {};
  variables.input.id = readOutData.id;
  return variables;
};

import { makeNetworkRequest } from '@isograph/react';
const resolver = (environment, artifact, readOutData, filteredVariables) => (mutationParams) => {
  const variables = includeReadOutData({...filteredVariables, ...mutationParams}, readOutData);
  makeNetworkRequest(environment, artifact, variables);
};


// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = any;

export type ReadFromStoreType = Pet__set_pet_tagline__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

export type Pet__set_pet_tagline__param = {
  id: string,
};

// The type, when returned from the resolver
export type ResolverReturnType = any;

const artifact: ReaderArtifact<ReadFromStoreType, Pet__set_pet_tagline__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
