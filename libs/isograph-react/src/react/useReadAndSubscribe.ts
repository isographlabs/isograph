import { useEffect, useState } from 'react';
import { subscribe } from '../core/cache';
import { GraphqlAggregateError, GraphqlError } from '../core/errors';
import {
  ExtractData,
  FragmentReference,
  stableIdForFragmentReference,
  type UnknownTReadFromStore,
} from '../core/FragmentReference';
import {
  NetworkRequestReaderOptions,
  readButDoNotEvaluate,
  WithEncounteredRecords,
} from '../core/read';
import type { ReaderAst } from '../core/reader';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { useRerenderOnChange } from './useRerenderOnChange';

/**
 * Read the data from a fragment reference and subscribe to updates.
 */
export function useReadAndSubscribe<
  TReadFromStore extends UnknownTReadFromStore,
>(
  fragmentReference: FragmentReference<TReadFromStore, any>,
  networkRequestOptions: NetworkRequestReaderOptions,
  readerAst: ReaderAst<TReadFromStore>,
): ExtractData<TReadFromStore> {
  const environment = useIsographEnvironment();
  const [readOutDataAndRecords, setReadOutDataAndRecords] = useState(() =>
    readButDoNotEvaluate(environment, fragmentReference, networkRequestOptions),
  );
  useRerenderOnChange(
    readOutDataAndRecords,
    fragmentReference,
    setReadOutDataAndRecords,
    readerAst,
  );

  if (readOutDataAndRecords.errors) {
    throw new GraphqlAggregateError(
      readOutDataAndRecords.errors.map((error) => new GraphqlError(error)),
    );
  }

  return readOutDataAndRecords.item;
}

export function useSubscribeToMultiple<
  TReadFromStore extends UnknownTReadFromStore,
>(
  items: ReadonlyArray<{
    records: WithEncounteredRecords<TReadFromStore>;
    callback: (updatedRecords: WithEncounteredRecords<TReadFromStore>) => void;
    fragmentReference: FragmentReference<TReadFromStore, any>;
    readerAst: ReaderAst<TReadFromStore>;
  }>,
) {
  const environment = useIsographEnvironment();
  useEffect(
    () => {
      const cleanupFns = items.map(
        ({ records, callback, fragmentReference, readerAst }) => {
          return subscribe(
            environment,
            records,
            fragmentReference,
            callback,
            readerAst,
          );
        },
      );
      return () => {
        cleanupFns.forEach((loader) => {
          loader();
        });
      };
    },
    // By analogy to useReadAndSubscribe, we can have an empty dependency array?
    // Maybe callback has to be depended on. I don't know!
    // TODO find out
    [
      items
        .map(({ fragmentReference }) => {
          stableIdForFragmentReference(fragmentReference);
        })
        .join('.'),
    ],
  );
}
