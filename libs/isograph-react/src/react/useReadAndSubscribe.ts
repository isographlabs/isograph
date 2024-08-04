import { useState } from 'react';
import { FragmentReference } from '../core/FragmentReference';
import { IsographEnvironment } from '../core/IsographEnvironment';
import {
  NetworkRequestReaderOptions,
  readButDoNotEvaluate,
} from '../core/read';
import { useRerenderOnChange } from './useRerenderOnChange';

/**
 * Read the data from a fragment reference and subscribe to updates.
 * Does not pass the data to the fragment reference's resolver function.
 */
export function useReadAndSubscribe<TReadFromStore extends Object>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<TReadFromStore, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
): TReadFromStore {
  const [readOutDataAndRecords, setReadOutDataAndRecords] = useState(() =>
    readButDoNotEvaluate(environment, fragmentReference, networkRequestOptions),
  );
  useRerenderOnChange(
    environment,
    readOutDataAndRecords,
    fragmentReference,
    setReadOutDataAndRecords,
  );
  return readOutDataAndRecords.item;
}
