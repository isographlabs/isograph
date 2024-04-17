import { useState } from 'react';
import { stableCopy } from './cache';
import { IsographEnvironment } from './IsographEnvironment';
import { readButDoNotEvaluate } from './read';
import { useRerenderOnChange } from './useRerenderOnChange';
import { FragmentReference } from './FragmentReference';

export function getOrCreateCachedComponent(
  environment: IsographEnvironment,
  componentName: string,
  fragmentReference: FragmentReference<any, any>,
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
        // During pre-commit renders, we call readButDoNotEvaluate.
        // There may be multiple pre-commit renders, so we should find
        // a way to read the cached data from the store instead
        const [readOutDataAndRecords, setReadOutDataAndRecords] = useState(() =>
          readButDoNotEvaluate(environment, fragmentReference),
        );

        useRerenderOnChange(
          environment,
          readOutDataAndRecords,
          fragmentReference,
          setReadOutDataAndRecords,
        );

        if (typeof window !== 'undefined' && window.__LOG) {
          console.log(
            'Component re-rendered: ' +
              componentName +
              ' ' +
              fragmentReference.root,
          );
        }

        return fragmentReference.readerArtifact.resolver(
          readOutDataAndRecords.item,
          additionalRuntimeProps,
        );
      }
      Component.displayName = `${componentName} (id: ${fragmentReference.root}) @component`;
      return Component;
    })();
  return byArgs[stringifiedArgs];
}
