import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { readButDoNotEvaluate } from './read';
import { FragmentReference } from './FragmentReference';
import { useState } from 'react';
import { useRerenderOnChange } from './useRerenderOnChange';
import { getOrCreateCachedComponent } from './componentCache';

export function useResult<TReadFromStore extends Object, TClientFieldValue>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>,
): TClientFieldValue {
  const environment = useIsographEnvironment();

  switch (fragmentReference.readerArtifact.variant.kind) {
    case 'Component': {
      // @ts-expect-error
      return getOrCreateCachedComponent(
        environment,
        fragmentReference.readerArtifact.variant.componentName,
        fragmentReference,
      );
    }
    case 'Eager': {
      const [readOutDataAndRecords, setReadOutDataAndRecords] = useState(() =>
        readButDoNotEvaluate(environment, fragmentReference),
      );
      useRerenderOnChange(
        environment,
        readOutDataAndRecords,
        fragmentReference,
        setReadOutDataAndRecords,
      );
      // @ts-expect-error
      return readOutDataAndRecords.item;
    }
  }
}
