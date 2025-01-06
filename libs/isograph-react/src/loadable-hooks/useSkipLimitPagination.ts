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
import { FetchOptions } from '../core/check';

type UseSkipLimitReturnValue<
  TReadFromStore extends { data: object; parameters: object },
  TItem,
> =
  | {
      readonly kind: 'Complete';
      readonly fetchMore: (
        count: number,
        fetchOptions?: FetchOptions<ReadonlyArray<TItem>>,
      ) => void;
      readonly results: ReadonlyArray<TItem>;
    }
  | {
      readonly kind: 'Pending';
      readonly results: ReadonlyArray<TItem>;
      readonly pendingFragment: FragmentReference<
        TReadFromStore,
        ReadonlyArray<TItem>
      >;
    };

type ArrayFragmentReference<
  TReadFromStore extends { parameters: object; data: object },
  TItem,
> = FragmentReference<TReadFromStore, ReadonlyArray<TItem>>;

type LoadedFragmentReferences<
  TReadFromStore extends { parameters: object; data: object },
  TItem,
> = ReadonlyArray<LoadedFragmentReference<TReadFromStore, TItem>>;

type LoadedFragmentReference<
  TReadFromStore extends { parameters: object; data: object },
  TItem,
> = ItemCleanupPair<
  ReferenceCountedPointer<ArrayFragmentReference<TReadFromStore, TItem>>
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

type UseSkipLimitPaginationArgs = {
  skip: number;
  limit: number;
};

export function useSkipLimitPagination<
  TItem,
  TReadFromStore extends {
    parameters: object;
    data: object;
  },
>(
  loadableField: LoadableField<
    TReadFromStore,
    ReadonlyArray<TItem>,
    UseSkipLimitPaginationArgs
  >,
  initialState?: {
    skip?: number | void | null;
  },
): UseSkipLimitReturnValue<TReadFromStore, TItem> {
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
  function readCompletedFragmentReferences(
    completedReferences: ArrayFragmentReference<TReadFromStore, TItem>[],
  ) {
    const results = completedReferences.map((fragmentReference, i) => {
      const readerWithRefetchQueries = readPromise(
        fragmentReference.readerWithRefetchQueries,
      );

      // invariant: readOutDataAndRecords.length === completedReferences.length
      const data = readOutDataAndRecords[i]?.item;
      if (data == null) {
        throw new Error(
          'Parameter data is unexpectedly null. This is indicative of a bug in Isograph.',
        );
      }

      const firstParameter = {
        data,
        parameters: fragmentReference.variables,
      };

      if (
        readerWithRefetchQueries.readerArtifact.kind !== 'EagerReaderArtifact'
      ) {
        throw new Error(
          `@loadable field of kind "${readerWithRefetchQueries.readerArtifact.kind}" is not supported by useSkipLimitPagination`,
        );
      }

      return readerWithRefetchQueries.readerArtifact.resolver(firstParameter);
    });

    const items = flatten(results);
    return items;
  }

  function subscribeCompletedFragmentReferences(
    completedReferences: ArrayFragmentReference<TReadFromStore, TItem>[],
  ) {
    return completedReferences.map(
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

        const records = readOutDataAndRecords[i];
        if (records == null) {
          throw new Error(
            'subscribeCompletedFragmentReferences records is unexpectedly null',
          );
        }

        return {
          fragmentReference,
          readerAst: readerWithRefetchQueries.readerArtifact.readerAst,
          records,
          callback(_data) {
            rerender({});
          },
        };
      },
    );
  }

  const getFetchMore =
    (loadedSoFar: number) =>
    (
      count: number,
      fetchOptions?: FetchOptions<ReadonlyArray<TItem>>,
    ): void => {
      const loadedField = loadableField(
        {
          skip: loadedSoFar,
          limit: count,
        },
        fetchOptions ?? {},
      )[1]();
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

  const mostRecentItem:
    | LoadedFragmentReference<TReadFromStore, TItem>
    | undefined = loadedReferences[loadedReferences.length - 1];
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

  const slicedFragmentReferences =
    networkRequestStatus?.kind === 'Ok'
      ? loadedReferences
      : loadedReferences.slice(0, loadedReferences.length - 1);

  const completedFragmentReferences = slicedFragmentReferences.map(
    ([pointer]) => {
      const fragmentReference = pointer.getItemIfNotDisposed();
      if (fragmentReference == null) {
        throw new Error(
          'FragmentReference is unexpectedly disposed. \
            This is indicative of a bug in Isograph.',
        );
      }
      return fragmentReference;
    },
  );

  const readOutDataAndRecords = completedFragmentReferences.map(
    (fragmentReference) =>
      readButDoNotEvaluate(
        environment,
        fragmentReference,
        networkRequestOptions,
      ),
  );

  useSubscribeToMultiple<TReadFromStore>(
    subscribeCompletedFragmentReferences(completedFragmentReferences),
  );

  if (!networkRequestStatus) {
    return {
      kind: 'Complete',
      fetchMore: getFetchMore(initialState?.skip ?? 0),
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
        results: readCompletedFragmentReferences(completedFragmentReferences),
      };
    }
    case 'Err': {
      throw networkRequestStatus.error;
    }
    case 'Ok': {
      const results = readCompletedFragmentReferences(
        completedFragmentReferences,
      );
      return {
        kind: 'Complete',
        results,
        fetchMore: getFetchMore(results.length),
      };
    }
  }
}

// @ts-ignore
function tsTests() {
  type Parameters = {
    readonly search: string;
    readonly skip: number;
    readonly limit: number;
  };

  let basicLoadable!: LoadableField<
    {
      readonly data: object;
      readonly parameters: Omit<Parameters, 'search'>;
    },
    object[]
  >;

  useSkipLimitPagination(basicLoadable);
  useSkipLimitPagination(basicLoadable, {});
  useSkipLimitPagination(basicLoadable, { skip: 10 });

  let unprovidedSearchLoadable!: LoadableField<
    {
      readonly data: object;
      readonly parameters: Parameters;
    },
    object[]
  >;
  // @ts-expect-error
  useSkipLimitPagination(unprovidedSearchLoadable);

  let providedSearchLoadable!: LoadableField<
    {
      readonly data: object;
      readonly parameters: Parameters;
    },
    object[],
    Omit<Parameters, 'search'>
  >;

  useSkipLimitPagination(providedSearchLoadable);
}
