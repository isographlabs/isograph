import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Pet____refetch__param } from './param_type.ts';
import { Pet____refetch__outputType } from './output_type.ts';
import { makeNetworkRequest, type IsographEnvironment, type IsographEntrypoint } from '@isograph/react';
const resolver = (
  environment: IsographEnvironment,
  artifact: IsographEntrypoint<any, any>,
  variables: any
) => () => makeNetworkRequest(environment, artifact, variables);

const readerAst: ReaderAst<Pet____refetch__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
];

const artifact: ReaderArtifact<
  Pet____refetch__param,
  Pet____refetch__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "__refetch",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Eager" },
};

export default artifact;
