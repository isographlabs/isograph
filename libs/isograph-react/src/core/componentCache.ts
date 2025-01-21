import { useReadAndSubscribe } from '../react/useReadAndSubscribe';
import { stableCopy } from './cache';
import { FragmentReference } from './FragmentReference';
import { IsographEnvironment } from './IsographEnvironment';
import { logMessage } from './logging';
import { readPromise } from './PromiseWrapper';
import { NetworkRequestReaderOptions } from './read';

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

  const recordLink = fragmentReference.root.__link;

  const componentsByName = (cachedComponentsById[recordLink] ??= {});

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

        logMessage(environment, {
          kind: 'ComponentRerendered',
          componentName,
          rootLink: fragmentReference.root,
        });

        return readerWithRefetchQueries.readerArtifact.resolver(
          {
            data,
            parameters: fragmentReference.variables,
          },
          additionalRuntimeProps,
        );
      }
      Component.displayName = `${componentName} (id: ${fragmentReference.root}) @component`;
      return Component;
    })();
  return byArgs[stringifiedArgs];
}
