import { ItemCleanupPair } from '@isograph/disposable-types';
import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import {
  createReferenceCountedPointer,
  ReferenceCountedPointer,
} from '@isograph/reference-counted-pointer';
import { useState } from 'react';
import { subscribeToAnyChange } from '../core/cache';
import { FetchOptions } from '../core/check';
import {
  FragmentReference,
  type UnknownTReadFromStore,
} from '../core/FragmentReference';
import { getPromiseState, readPromise } from '../core/PromiseWrapper';
import {
  readButDoNotEvaluate,
  type WithEncounteredRecords,
} from '../core/read';
import { LoadableField, type ReaderAst } from '../core/reader';
import { getOrCreateCachedStartUpdate } from '../core/startUpdate';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { useSubscribeToMultiple } from '../react/useReadAndSubscribe';
import { maybeUnwrapNetworkRequest } from '../react/useResult';

export type UsePaginationReturnValue<
  TReadFromStore extends UnknownTReadFromStore,
  TItem,
> =
  | {
      kind: 'Pending';
      pendingFragment: FragmentReference<TReadFromStore, Connection<TItem>>;
      results: ReadonlyArray<TItem>;
    }
  | {
      kind: 'Complete';
      fetchMore: (
        count: number,
        fetchOptions?: FetchOptions<Connection<TItem>>,
      ) => void;
      results: ReadonlyArray<TItem>;
      hasNextPage: boolean;
    };

type LoadedFragmentReferences<
  TReadFromStore extends { parameters: object; data: object },
  TItem,
> = ReadonlyArray<LoadedFragmentReference<TReadFromStore, TItem>>;

type LoadedFragmentReference<
  TReadFromStore extends { parameters: object; data: object },
  TItem,
> = ItemCleanupPair<
  ReferenceCountedPointer<FragmentReference<TReadFromStore, TItem>>
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

export type PageInfo = {
  readonly hasNextPage: boolean;
  readonly endCursor: string | null;
};

export type Connection<T> = {
  readonly edges: ReadonlyArray<T> | null;
  readonly pageInfo: PageInfo;
};

type NonNullConnection<T> = {
  readonly edges: ReadonlyArray<T>;
  readonly pageInfo: PageInfo;
};

export type UseConnectionSpecPaginationArgs = {
  first: number;
  after: string | null;
};

export function useConnectionSpecPagination<
  TReadFromStore extends UnknownTReadFromStore,
  TItem,
>(
  loadableField: LoadableField<
    TReadFromStore,
    Connection<TItem>,
    UseConnectionSpecPaginationArgs
  >,
  initialState?: PageInfo,
): UsePaginationReturnValue<TReadFromStore, TItem> {
  const networkRequestOptions = {
    suspendIfInFlight: true,
    throwOnNetworkError: true,
  };
  const { state, setState } =
    useUpdatableDisposableState<
      LoadedFragmentReferences<TReadFromStore, Connection<TItem>>
    >();

  const environment = useIsographEnvironment();

  // TODO move this out of useSkipLimitPagination, and pass environment and networkRequestOptions
  // as parameters (or recreate networkRequestOptions)
  function readCompletedFragmentReferences(
    completedReferences: FragmentReference<TReadFromStore, Connection<TItem>>[],
  ): NonNullConnection<TItem> {
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
        ...(readerWithRefetchQueries.readerArtifact.hasUpdatable
          ? {
              startUpdate: getOrCreateCachedStartUpdate(
                environment,
                fragmentReference,
                readerWithRefetchQueries.readerArtifact.fieldName,
                networkRequestOptions,
              ),
            }
          : undefined),
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

    const items = flatten(results.map((result) => result.edges ?? []));

    return {
      edges: items,
      pageInfo: results[results.length - 1]?.pageInfo ?? {
        endCursor: null,
        hasNextPage: true,
      },
    };
  }

  function subscribeCompletedFragmentReferences(
    completedReferences: FragmentReference<TReadFromStore, Connection<TItem>>[],
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
        fragmentReference: FragmentReference<TReadFromStore, Connection<TItem>>;
        readerAst: ReaderAst<Connection<TItem>>;
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
    (after: string | null) =>
    (count: number, fetchOptions?: FetchOptions<Connection<TItem>>): void => {
      const loadedField = loadableField(
        {
          after: after,
          first: count,
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
              FragmentReference<TReadFromStore, Connection<TItem>>
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
    | LoadedFragmentReference<TReadFromStore, Connection<TItem>>
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
      fetchMore: getFetchMore(initialState?.endCursor ?? null),
      results: [],
      hasNextPage: initialState?.hasNextPage ?? true,
    };
  }

  switch (networkRequestStatus.kind) {
    case 'Pending': {
      const unsubscribe = subscribeToAnyChange(environment, () => {
        unsubscribe();
        rerender({});
      });

      const results = readCompletedFragmentReferences(
        completedFragmentReferences,
      );
      return {
        results: results.edges,
        kind: 'Pending',
        pendingFragment: mostRecentFragmentReference,
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
        results: results.edges,
        hasNextPage: results.pageInfo.hasNextPage,
        kind: 'Complete',
        fetchMore: getFetchMore(results.pageInfo.endCursor),
      };
    }
  }
}
