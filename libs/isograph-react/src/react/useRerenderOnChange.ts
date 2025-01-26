import { useEffect } from 'react';
import { subscribe } from '../core/cache';
import { FragmentReference } from '../core/FragmentReference';
import { WithEncounteredRecords } from '../core/read';
import type { ReaderAst } from '../core/reader';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

// TODO add unit tests for this. Add integration tests that test
// behavior when the encounteredRecords underneath a fragment change.
export function useRerenderOnChange<
  TReadFromStore extends { parameters: object; data: object },
>(
  encounteredDataAndRecords: WithEncounteredRecords<TReadFromStore>,
  fragmentReference: FragmentReference<any, any>,
  setEncounteredDataAndRecords: (
    data: WithEncounteredRecords<TReadFromStore>,
  ) => void,
  readerAst: ReaderAst<TReadFromStore>,
) {
  const environment = useIsographEnvironment();
  useEffect(() => {
    return subscribe(
      environment,
      encounteredDataAndRecords,
      fragmentReference,
      (newEncounteredDataAndRecords) => {
        setEncounteredDataAndRecords(newEncounteredDataAndRecords);
      },
      readerAst,
    );
    // Note: this is an empty array on purpose:
    // - the fragment reference is stable for the life of the component
    // - ownership of encounteredDataAndRecords is transferred into the
    //   environment
    // - though maybe we need to include setEncounteredDataAndRecords in
    //   the dependency array
  }, []);
}
