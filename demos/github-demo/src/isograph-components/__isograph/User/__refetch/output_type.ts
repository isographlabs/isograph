import { RefetchQueryNormalizationArtifact } from '@isograph/react';
import { makeNetworkRequest, type IsographEnvironment } from '@isograph/react';
const resolver = (
  environment: IsographEnvironment,
  artifact: RefetchQueryNormalizationArtifact,
  variables: any
) => () => makeNetworkRequest(environment, artifact, variables);
export type User____refetch__outputType = () => void;