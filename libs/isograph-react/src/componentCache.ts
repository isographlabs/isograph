import { stableCopy } from './cache';
import { RefetchQueryArtifactWrapper } from './entrypoint';
import { IsographEnvironment, DataId } from './IsographEnvironment';
import { readButDoNotEvaluate } from './read';
import { ReaderArtifact } from './reader';
import { useRerenderWhenEncounteredRecordChanges } from './useRerenderWhenEncounteredRecordChanges';

export function getOrCreateCachedComponent(
  environment: IsographEnvironment,
  rootId: DataId,
  componentName: string,
  readerArtifact: ReaderArtifact<any, any>,
  variables: { [key: string]: string },
  resolverRefetchQueries: RefetchQueryArtifactWrapper[],
): React.FC<any> {
  // cachedComponentsById is a three layer cache: id, then component name, then
  // stringified args. These three, together, uniquely identify a read at a given
  // time.
  const cachedComponentsById = environment.componentCache;

  cachedComponentsById[rootId] = cachedComponentsById[rootId] ?? {};
  const componentsByName = cachedComponentsById[rootId];

  componentsByName[componentName] = componentsByName[componentName] ?? {};
  const byArgs = componentsByName[componentName];

  const stringifiedArgs = JSON.stringify(stableCopy(variables));
  byArgs[stringifiedArgs] =
    byArgs[stringifiedArgs] ??
    (() => {
      function Component(additionalRuntimeProps: { [key: string]: any }) {
        const { item: data, encounteredRecords } = readButDoNotEvaluate(
          environment,
          {
            kind: 'FragmentReference',
            readerArtifact: readerArtifact,
            root: rootId,
            variables,
            nestedRefetchQueries: resolverRefetchQueries,
          },
        );

        useRerenderWhenEncounteredRecordChanges(
          environment,
          encounteredRecords,
        );

        if (typeof window !== 'undefined' && window.__LOG) {
          console.log('Component re-rendered: ' + componentName + ' ' + rootId);
        }

        return readerArtifact.resolver(data, additionalRuntimeProps);
      }
      Component.displayName = `${componentName} (id: ${rootId}) @component`;
      return Component;
    })();
  return byArgs[stringifiedArgs];
}
