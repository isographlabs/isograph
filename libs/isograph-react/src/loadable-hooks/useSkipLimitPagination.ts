import { LoadableField, type ReaderAst } from '../core/reader';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { ItemCleanupPair } from '@isograph/disposable-types';
import { FragmentReference } from '../core/FragmentReference';
import { maybeUnwrapNetworkRequest } from '../react/useResult';
import { readButDoNotEvaluate } from '../core/read';
import { subscribeToAnyChange } from '../core/cache';
import { useState } from 'react';
import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import {
  createReferenceCountedPointer,
  ReferenceCountedPointer,
} from '@isograph/reference-counted-pointer';
import { getPromiseState, readPromise } from '../core/PromiseWrapper';
import { type WithEncounteredRecords } from '../core/read';
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

type ArrayFragmentReference<
  TReadFromStore extends Object,
  TItem,
> = FragmentReference<TReadFromStore, ReadonlyArray<TItem>>;

type LoadedFragmentReference<
  TReadFromStore extends Object,
  TItem,
> = ItemCleanupPair<
  ReferenceCountedPointer<ArrayFragmentReference<TReadFromStore, TItem>>
>;

type LoadedFragmentReferences<
  TReadFromStore extends Object,
  TItem,
> = ReadonlyArray<LoadedFragmentReference<TReadFromStore, TItem>>;

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
  TItem,
  TReadFromStore extends Object,
>(
  loadableField: LoadableField<TArgs, ReadonlyArray<TItem>>,
): UseSkipLimitReturnValue<TArgs, TItem> {
  const networkRequestOptions = {
    suspendIfInFlight: true,
    throwOnNetworkError: true,
  };
  const { state, setState } =
    useUpdatableDisposableState<
      LoadedFragmentReferences<TReadFromStore, TItem>
    >();

  const environment = useIsographEnvironment();

  // TODO move this out of useSkipLimitPagination, and pass environment and networkRequestOptions
  // as parameters (or recreate networkRequestOptions)
  function subscribeCompletedFragmentReferences(
    completedReferences: ArrayFragmentReference<TReadFromStore, TItem>[],
  ) {
    // In general, this will not suspend. But it could, if there is missing data.
    // A better version of this hook would not do any reading here.
    const results = completedReferences.map(
      (
        fragmentReference,
        i,
      ): {
        records: WithEncounteredRecords<TReadFromStore>;
        callback: (
          updatedRecords: WithEncounteredRecords<TReadFromStore>,
        ) => void;
        fragmentReference: ArrayFragmentReference<TReadFromStore, TItem>;
        readerAst: ReaderAst<TItem>;
      } => {
        maybeUnwrapNetworkRequest(
          fragmentReference.networkRequest,
          networkRequestOptions,
        );

        const readerWithRefetchQueries = readPromise(
          fragmentReference.readerWithRefetchQueries,
        );

        return {
          fragmentReference,
          readerAst: readerWithRefetchQueries.readerArtifact.readerAst,
          records: readOutDataAndRecords[i],
          callback(data) {
            setReadOutDataAndRecords((current) => {
              const next = [...current];
              next[i] = data;
              return next;
            });
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
            ReferenceCountedPointer<
              ArrayFragmentReference<TReadFromStore, TItem>
            >
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

  const [, rerender] = useState({});

  const loadedReferences = state === UNASSIGNED_STATE ? [] : state;

  const mostRecentItem: LoadedFragmentReference<TReadFromStore, TItem> | null =
    loadedReferences[loadedReferences.length - 1];
  const mostRecentFragmentReference =
    mostRecentItem?.[0].getItemIfNotDisposed();

  if (mostRecentItem && mostRecentFragmentReference === null) {
    throw new Error(
      'FragmentReference is unexpectedly disposed. \
      This is indicative of a bug in Isograph.',
    );
  }

  const networkRequestStatus =
    mostRecentFragmentReference &&
    getPromiseState(mostRecentFragmentReference.networkRequest);

  const completedFragmentReferences =
    networkRequestStatus?.kind === 'Ok'
      ? loadedReferences
      : loadedReferences.slice(0, loadedReferences.length - 1);

  const [readOutDataAndRecords, setReadOutDataAndRecords] = useState<
    WithEncounteredRecords<TReadFromStore>[]
  >([]);

  const fragmentReferences = completedFragmentReferences.map(([pointer]) => {
    const fragmentReference = pointer.getItemIfNotDisposed();
    if (fragmentReference == null) {
      throw new Error(
        'FragmentReference is unexpectedly disposed. \
        This is indicative of a bug in Isograph.',
      );
    }
    return fragmentReference;
  });

  useSubscribeToMultiple<TReadFromStore>(
    subscribeCompletedFragmentReferences(fragmentReferences),
  );

  const results = flatten(
    fragmentReferences.map((fragmentReference, i) => {
      const readerWithRefetchQueries = readPromise(
        fragmentReference.readerWithRefetchQueries,
      );

      if (
        readerWithRefetchQueries.readerArtifact.kind !== 'EagerReaderArtifact'
      ) {
        throw new Error(
          `@loadable field of kind "${readerWithRefetchQueries.readerArtifact.kind}" is not supported by useSkipLimitPagination`,
        );
      }

      return readerWithRefetchQueries.readerArtifact.resolver({
        data: readOutDataAndRecords[i].item,
        parameters: fragmentReference.variables,
      });
    }),
  );

  if (!networkRequestStatus) {
    return {
      kind: 'Complete',
      fetchMore: getFetchMore(0),
      results: [],
    };
  }

  switch (networkRequestStatus.kind) {
    case 'Pending': {
      const unsubscribe = subscribeToAnyChange(environment, () => {
        unsubscribe();
        rerender({});
      });

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
      return {
        kind: 'Complete',
        results,
        fetchMore: getFetchMore(results.length),
      };
    }
  }
}
