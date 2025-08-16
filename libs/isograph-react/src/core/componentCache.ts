import {
  FragmentReference,
  stableIdForFragmentReference,
} from './FragmentReference';
import { IsographEnvironment } from './IsographEnvironment';
import { NetworkRequestReaderOptions } from './read';
import { createStartUpdate } from './startUpdate';

export function getOrCreateCachedComponent<TComponent>(
  environment: IsographEnvironment<TComponent>,
  componentName: string,
  fragmentReference: FragmentReference<any, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
): TComponent {
  // We create startUpdate outside of component to make it stable
  const startUpdate = createStartUpdate(environment, fragmentReference);

  return (environment.componentCache[
    stableIdForFragmentReference(fragmentReference, componentName)
  ] ??= environment.componentFunction(
    environment,
    componentName,
    fragmentReference,
    networkRequestOptions,
    startUpdate,
  ));
}
