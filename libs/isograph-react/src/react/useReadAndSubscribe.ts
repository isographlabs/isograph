import { useState } from 'react';
import { FragmentReference } from '../core/FragmentReference';
import {
  NetworkRequestReaderOptions,
  readButDoNotEvaluate,
} from '../core/read';
import { useRerenderOnChange } from './useRerenderOnChange';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

/**
 * Read the data from a fragment reference and subscribe to updates.
 * Does not pass the data to the fragment reference's resolver function.
 */
export function useReadAndSubscribe<TReadFromStore extends Object>(
  fragmentReference: FragmentReference<TReadFromStore, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
): TReadFromStore {
  const environment = useIsographEnvironment();
  const [readOutDataAndRecords, setReadOutDataAndRecords] = useState(() =>
    readButDoNotEvaluate(environment, fragmentReference, networkRequestOptions),
  );
  useRerenderOnChange(
    readOutDataAndRecords,
    fragmentReference,
    setReadOutDataAndRecords,
  );
  return readOutDataAndRecords.item;
}
