import { useEffect, useState } from 'react';
import {
  type ExtractData,
  type FragmentReference,
  stableIdForFragmentReference,
  type UnknownTReadFromStore,
} from '../core/FragmentReference';
import type { IsographComponentFunction } from '../core/IsographEnvironment';
import { readPromise } from '../core/PromiseWrapper';
import { logMessage } from '../core/logging';
import {
  type NetworkRequestReaderOptions,
  readButDoNotEvaluate,
  type WithEncounteredRecords,
} from '../core/read';
import type { ReaderAst } from '../core/reader';
import { subscribe } from '../core/subscribe';
import { useIsographEnvironment } from './IsographEnvironmentProvider';
import { maybeUnwrapNetworkRequest } from './maybeUnwrapNetworkRequest';
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

  if (readOutDataAndRecords.kind === 'Errors') {
    throw readOutDataAndRecords.errors;
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

export const componentFunction: IsographComponentFunction = (
  environment,
  fragmentReference,
  networkRequestOptions,
  startUpdate,
) => {
  function Component(additionalRuntimeProps: { [key: string]: any }) {
    maybeUnwrapNetworkRequest(
      fragmentReference.networkRequest,
      networkRequestOptions,
    );
    const readerWithRefetchQueries = readPromise(
      fragmentReference.readerWithRefetchQueries,
    );

    const data = useReadAndSubscribe(
      fragmentReference,
      networkRequestOptions,
      readerWithRefetchQueries.readerArtifact.readerAst,
    );

    logMessage(environment, () => ({
      kind: 'ComponentRerendered',
      componentName: fragmentReference.fieldName,
      rootLink: fragmentReference.root,
    }));

    return readerWithRefetchQueries.readerArtifact.resolver(
      // @ts-expect-error
      {
        data,
        parameters: fragmentReference.variables,
        startUpdate: readerWithRefetchQueries.readerArtifact.hasUpdatable
          ? startUpdate
          : undefined,
      },
      additionalRuntimeProps,
    );
  }
  const idString = `(type: ${fragmentReference.root.__typename}, id: ${fragmentReference.root.__link})`;
  Component.displayName = `${fragmentReference.fieldName} ${idString} @component`;
  return Component;
};
