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
export type Pet__set_pet_tagline__outputType = (params: any) => void;

const readerAst: ReaderAst<Pet__set_pet_tagline__param> = [
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

const artifact: ReaderArtifact<
  Pet__set_pet_tagline__param,
  Pet__set_pet_tagline__param,
  Pet__set_pet_tagline__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
