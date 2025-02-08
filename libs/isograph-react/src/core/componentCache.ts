import { useReadAndSubscribe } from '../react/useReadAndSubscribe';
import {
  FragmentReference,
  stableIdForFragmentReference,
} from './FragmentReference';
import { IsographEnvironment } from './IsographEnvironment';
import { logMessage } from './logging';
import { readPromise } from './PromiseWrapper';
import { NetworkRequestReaderOptions } from './read';
import { startUpdate } from './startUpdate';

export function getOrCreateCachedComponent(
  environment: IsographEnvironment,
  componentName: string,
  fragmentReference: FragmentReference<any, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
): React.FC<any> {
  const cachedComponentsByStableFragmentReferenceId =
    environment.componentCache;

  const componentsByName = (cachedComponentsByStableFragmentReferenceId[
    stableIdForFragmentReference(fragmentReference)
  ] ??= {});

  return (componentsByName[componentName] ??= (() => {
    function Component(additionalRuntimeProps: { [key: string]: any }) {
      const readerWithRefetchQueries = readPromise(
        fragmentReference.readerWithRefetchQueries,
      );

      const data = useReadAndSubscribe(
        fragmentReference,
        networkRequestOptions,
        readerWithRefetchQueries.readerArtifact.readerAst,
      );

      logMessage(environment, {
        kind: 'ComponentRerendered',
        componentName,
        rootLink: fragmentReference.root,
      });

      return readerWithRefetchQueries.readerArtifact.resolver(
        {
          data,
          parameters: fragmentReference.variables,
          startUpdate: readerWithRefetchQueries.readerArtifact.hasUpdatable
            ? startUpdate(environment, data)
            : undefined,
        },
        additionalRuntimeProps,
      );
    }
    Component.displayName = `${componentName} (id: ${fragmentReference.root}) @component`;
    return Component;
  })());
}
