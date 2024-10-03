import { stableCopy } from './cache';
import { IsographEnvironment } from './IsographEnvironment';
import { FragmentReference } from './FragmentReference';
import { useReadAndSubscribe } from '../react/useReadAndSubscribe';
import { NetworkRequestReaderOptions } from './read';
import { readPromise } from './PromiseWrapper';

export function getOrCreateCachedComponent(
  environment: IsographEnvironment,
  componentName: string,
  fragmentReference: FragmentReference<any, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
): React.FC<any> {
  // cachedComponentsById is a three layer cache: id, then component name, then
  // stringified args. These three, together, uniquely identify a read at a given
  // time.
  const cachedComponentsById = environment.componentCache;

  cachedComponentsById[fragmentReference.root] =
    cachedComponentsById[fragmentReference.root] ?? {};
  const componentsByName = cachedComponentsById[fragmentReference.root];

  componentsByName[componentName] = componentsByName[componentName] ?? {};
  const byArgs = componentsByName[componentName];

  const stringifiedArgs = JSON.stringify(
    stableCopy(fragmentReference.variables),
  );
  byArgs[stringifiedArgs] =
    byArgs[stringifiedArgs] ??
    (() => {
      function Component(additionalRuntimeProps: { [key: string]: any }) {
        const readerWithRefetchQueries = readPromise(
          fragmentReference.readerWithRefetchQueries,
        );

        const data = useReadAndSubscribe(
          fragmentReference,
          networkRequestOptions,
          readerWithRefetchQueries.readerArtifact.readerAst,
        );

        // @ts-expect-error
        if (typeof window !== 'undefined' && window.__LOG) {
          console.log(
            'Component re-rendered: ' +
              componentName +
              ' ' +
              fragmentReference.root,
          );
        }

        const firstParameter = {
          data,
          parameters: fragmentReference.variables,
        };

        return readerWithRefetchQueries.readerArtifact.resolver(
          firstParameter,
          additionalRuntimeProps,
        );
      }
      Component.displayName = `${componentName} (id: ${fragmentReference.root}) @component`;
      return Component;
    })();
  return byArgs[stringifiedArgs];
}
