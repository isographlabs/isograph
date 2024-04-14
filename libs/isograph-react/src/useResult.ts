import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { read } from './read';
import { FragmentReference } from './FragmentReference';
import { useRerenderWhenEncounteredRecordChanges } from './useRerenderWhenEncounteredRecordChanges';

export function useResult<TReadFromStore extends Object, TClientFieldValue>(
  fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>,
): TClientFieldValue {
  const environment = useIsographEnvironment();

  const { item: data, encounteredRecords } = read(
    environment,
    fragmentReference,
  );

  useRerenderWhenEncounteredRecordChanges(environment, encounteredRecords);

  return data;
}
