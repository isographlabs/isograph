import { LoadableField, type ReaderAst } from '../core/reader';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { ItemCleanupPair } from '@isograph/disposable-types';
import { FragmentReference } from '../core/FragmentReference';
import { maybeUnwrapNetworkRequest } from '../react/useResult';
import {
  readButDoNotEvaluate,
  type WithEncounteredRecords,
} from '../core/read';
import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import {
  createReferenceCountedPointer,
  ReferenceCountedPointer,
} from '@isograph/reference-counted-pointer';
import { useRef, useState } from 'react';
import { getPromiseState, readPromise } from '../core/PromiseWrapper';
import { useSubscribeToMultiple } from '../react/useReadAndSubscribe';

type SkipOrLimit = 'skip' | 'limit';
type OmitSkipLimit<TArgs> = keyof Omit<TArgs, SkipOrLimit> extends never
  ? void | Record<string, never>
  : Omit<TArgs, SkipOrLimit>;

type UseSkipLimitReturnValue<TArgs, TItem> =
  | {
      readonly kind: 'Complete';
      readonly fetchMore: (args: OmitSkipLimit<TArgs>, count: number) => void;
      readonly results: ReadonlyArray<TItem>;
    }
  | {
      readonly kind: 'Pending';
      readonly results: ReadonlyArray<TItem>;
      readonly pendingFragment: FragmentReference<any, ReadonlyArray<TItem>>;
    };

type ArrayFragmentReference<TItem> = FragmentReference<
  any,
  ReadonlyArray<TItem>
>;

type LoadedFragmentReference<TItem> = ItemCleanupPair<
  ReferenceCountedPointer<ArrayFragmentReference<TItem>>
>;

type LoadedFragmentReferences<TItem> = ReadonlyArray<
  LoadedFragmentReference<TItem>
>;

function flatten<T>(arr: ReadonlyArray<ReadonlyArray<T>>): ReadonlyArray<T> {
  let outArray: Array<T> = [];
  for (const subarr of arr) {
    for (const item of subarr) {
      outArray.push(item);
    }
  }
  return outArray;
}

export function useSkipLimitPagination<
  TArgs extends {
    skip: number | void | null;
    limit: number | void | null;
  },
  TItem extends object,
>(
  loadableField: LoadableField<TArgs, Array<TItem>>,
): UseSkipLimitReturnValue<TArgs, TItem> {
  const networkRequestOptions = {
    suspendIfInFlight: true,
    throwOnNetworkError: true,
  };
  const { state, setState } =
    useUpdatableDisposableState<LoadedFragmentReferences<TItem>>();

  const environment = useIsographEnvironment();

  // TODO move this out of useSkipLimitPagination, and pass environment and networkRequestOptions
  // as parameters (or recreate networkRequestOptions)
  function subscribeCompletedFragmentReferences(
    completedReferences: ReadonlyArray<
      ItemCleanupPair<ReferenceCountedPointer<ArrayFragmentReference<TItem>>>
    >,
  ) {
    // In general, this will not suspend. But it could, if there is missing data.
    // A better version of this hook would not do any reading here.
    const results = completedReferences.map(
      (
        [pointer],
        i,
      ): {
        records: WithEncounteredRecords<TItem>;
        callback: (updatedRecords: WithEncounteredRecords<TItem>) => void;
        fragmentReference: FragmentReference<TItem, any>;
        readerAst: ReaderAst<TItem>;
      } => {
        const fragmentReference = pointer.getItemIfNotDisposed();
        if (fragmentReference == null) {
          throw new Error(
            'FragmentReference is unexpectedly disposed. \
          This is indicative of a bug in Isograph.',
          );
        }

        maybeUnwrapNetworkRequest(
          fragmentReference.networkRequest,
          networkRequestOptions,
        );
        const data = readButDoNotEvaluate(
          environment,
          fragmentReference,
          networkRequestOptions,
        );

        const readerWithRefetchQueries = readPromise(
          fragmentReference.readerWithRefetchQueries,
        );

        if (!readOutDataAndRecordsRef.current[i]) {
          readOutDataAndRecordsRef.current[i] = {
            records: data,
            results: readerWithRefetchQueries.readerArtifact.resolver(
              data.item,
              undefined,
            ) as ReadonlyArray<any>,
          };
        }

        return {
          fragmentReference,
          readerAst: readerWithRefetchQueries.readerArtifact.readerAst,
          records: readOutDataAndRecordsRef.current[i].records,
          callback(data) {
            readOutDataAndRecordsRef.current[i] = {
              records: data,
              results: readerWithRefetchQueries.readerArtifact.resolver(
                data.item,
                undefined,
              ) as ReadonlyArray<any>,
            };
            rerender({});
          },
        };
      },
    );

    return results;
  }

  const getFetchMore =
    (loadedSoFar: number) =>
    (args: OmitSkipLimit<TArgs>, count: number): void => {
      // @ts-expect-error
      const loadedField = loadableField({
        ...args,
        skip: loadedSoFar,
        limit: count,
      })[1]();
      const newPointer = createReferenceCountedPointer(loadedField);
      const clonedPointers = loadedReferences.map(([refCountedPointer]) => {
        const clonedRefCountedPointer = refCountedPointer.cloneIfNotDisposed();
        if (clonedRefCountedPointer == null) {
          throw new Error(
            'This reference counted pointer has already been disposed. \
            This is indicative of a bug in useSkipLimitPagination.',
          );
        }
        return clonedRefCountedPointer;
      });
      clonedPointers.push(newPointer);

      const totalItemCleanupPair: ItemCleanupPair<
        ReadonlyArray<
          ItemCleanupPair<
            ReferenceCountedPointer<ArrayFragmentReference<TItem>>
          >
        >
      > = [
        clonedPointers,
        () => {
          clonedPointers.forEach(([, dispose]) => {
            dispose();
          });
        },
      ];

      setState(totalItemCleanupPair);
    };

  const loadedReferences = state === UNASSIGNED_STATE ? [] : state;

  const mostRecentItem: LoadedFragmentReference<TItem> | null =
    loadedReferences[loadedReferences.length - 1];
  const mostRecentFragmentReference =
    mostRecentItem?.[0].getItemIfNotDisposed();

  const networkRequestStatus =
    mostRecentFragmentReference &&
    getPromiseState(mostRecentFragmentReference.networkRequest);

  const completedFragmentReferences =
    networkRequestStatus?.kind === 'Ok'
      ? loadedReferences
      : loadedReferences.slice(0, loadedReferences.length - 1);

  const readOutDataAndRecordsRef = useRef<
    {
      records: WithEncounteredRecords<TItem>;
      results: ReadonlyArray<any>;
    }[]
  >([]);

  const [, rerender] = useState<{}>({});

  useSubscribeToMultiple<TItem>(
    subscribeCompletedFragmentReferences(completedFragmentReferences),
  );

  if (!mostRecentFragmentReference) {
    return {
      kind: 'Complete',
      fetchMore: getFetchMore(0),
      results: [],
    };
  }

  if (networkRequestStatus === null) {
    throw new Error(
      'FragmentReference is unexpectedly disposed. \
      This is indicative of a bug in Isograph.',
    );
  }

  switch (networkRequestStatus.kind) {
    case 'Pending': {
      const results = flatten(
        readOutDataAndRecordsRef.current.map((data) => data.results),
      );
      return {
        kind: 'Pending',
        pendingFragment: mostRecentFragmentReference,
        results: results,
      };
    }
    case 'Err': {
      throw networkRequestStatus.error;
    }
    case 'Ok': {
      const results = flatten(
        readOutDataAndRecordsRef.current.map((data) => data.results),
      );
      return {
        kind: 'Complete',
        results,
        fetchMore: getFetchMore(results.length),
      };
    }
  }
}
