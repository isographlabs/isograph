import type { FragmentReference } from './FragmentReference';
import { stableIdForFragmentReference } from './FragmentReference';
import type { IsographEnvironment } from './IsographEnvironment';
import type { NetworkRequestReaderOptions } from './read';
import { createStartUpdate } from './startUpdate';

export function getOrCreateCachedComponent(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<any, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
): React.FC<any> {
  // We create startUpdate outside of component to make it stable
  const startUpdate = createStartUpdate(
    environment,
    fragmentReference,
    networkRequestOptions,
  );

  return (environment.componentCache[
    stableIdForFragmentReference(fragmentReference)
  ] ??= environment.componentFunction(
    environment,
    fragmentReference,
    networkRequestOptions,
    startUpdate,
  ));
}
