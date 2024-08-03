import { useEffect } from 'react';
import { IsographEnvironment } from '../core/IsographEnvironment';
import { subscribe } from '../core/cache';
import { WithEncounteredRecords } from '../core/read';
import { FragmentReference } from '../core/FragmentReference';

// TODO add unit tests for this. Add integration tests that test
// behavior when the encounteredRecords underneath a fragment change.
export function useRerenderOnChange<TReadFromStore extends Object>(
  environment: IsographEnvironment,
  encounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
  fragmentReference: FragmentReference<any, any>,
  setEncounteredDataAndRecords: (
    data: WithEncounteredRecords<TReadFromStore>,
  ) => void,
) {
  useEffect(() => {
    return subscribe(
      environment,
      encounteredDataAndRecords,
      fragmentReference,
      (newEncounteredDataAndRecords) => {
        setEncounteredDataAndRecords(newEncounteredDataAndRecords);
      },
    );
    // Note: this is an empty array on purpose:
    // - the fragment reference is stable for the life of the component
    // - ownership of encounteredDataAndRecords is transferred into the
    //   environment
    // - though maybe we need to include setEncounteredDataAndRecords in
    //   the dependency array
  }, []);
}
