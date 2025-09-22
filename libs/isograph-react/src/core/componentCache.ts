import { useReadAndSubscribe } from '../react/useReadAndSubscribe';
import {
  FragmentReference,
  stableIdForFragmentReference,
} from './FragmentReference';
import { IsographEnvironment } from './IsographEnvironment';
import { logMessage } from './logging';
import { readPromise } from './PromiseWrapper';
import { NetworkRequestReaderOptions } from './read';
import { createStartUpdate } from './startUpdate';

export function getOrCreateCachedComponent(
  environment: IsographEnvironment,
  componentName: string,
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
    stableIdForFragmentReference(fragmentReference, componentName)
  ] ??= (() => {
    function Component(additionalRuntimeProps: { [key: string]: any }) {
      const readerWithRefetchQueries = readPromise(
        fragmentReference.readerWithRefetchQueries,
      );

      const data = useReadAndSubscribe(
        fragmentReference,
        networkRequestOptions,
        readerWithRefetchQueries.readerArtifact.readerAst,
      );

      logMessage(environment, () => ({
        kind: 'ComponentRerendered',
        componentName,
        rootLink: fragmentReference.root,
      }));

      return readerWithRefetchQueries.readerArtifact.resolver(
        {
          data,
          parameters: fragmentReference.variables,
          startUpdate: readerWithRefetchQueries.readerArtifact.hasUpdatable
            ? startUpdate
            : undefined,
        },
        additionalRuntimeProps,
      );
    }
    const idString = `(type: ${fragmentReference.root.__typename}, id: ${fragmentReference.root.__link})`;
    Component.displayName = `${componentName} ${idString} @component`;
    return Component;
  })());
}
