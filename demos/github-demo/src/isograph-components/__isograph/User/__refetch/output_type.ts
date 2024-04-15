import type {ExtractSecondParam} from '@isograph/react';
import { makeNetworkRequest, type IsographEnvironment, type IsographEntrypoint } from '@isograph/react';
const resolver = (
  environment: IsographEnvironment,
  artifact: IsographEntrypoint<any, any>,
  variables: any
) => () => makeNetworkRequest(environment, artifact, variables);
// the type, when read out (either via useLazyReference or via graph)
export type User____refetch__outputType = () => void;
