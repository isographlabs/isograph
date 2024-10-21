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
import { FragmentReference } from '../core/FragmentReference';
import { getPromiseState, readPromise } from '../core/PromiseWrapper';
import {
  readButDoNotEvaluate,
  type WithEncounteredRecords,
} from '../core/read';
import { LoadableField, type ReaderAst } from '../core/reader';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { useSubscribeToMultiple } from '../react/useReadAndSubscribe';
import { maybeUnwrapNetworkRequest } from '../react/useResult';

type UsePaginationReturnValue<
  TReadFromStore extends { parameters: object; data: object },
  TItem,
> =
  | {
      kind: 'Pending';
      pendingFragment: FragmentReference<TReadFromStore, Connection<TItem>>;
      results: ReadonlyArray<TItem>;
    }
  | {
      kind: 'Complete';
      fetchMore: (count: number) => void;
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

type PageInfo = {
  readonly hasNextPage: boolean;
  readonly endCursor: string | null;
};

type Connection<T> = {
  readonly edges: ReadonlyArray<T> | null;
  readonly pageInfo: PageInfo;
};

type NonNullConnection<T> = {
  readonly edges: ReadonlyArray<T>;
  readonly pageInfo: PageInfo;
};

type UseConnectionSpecPaginationArgs = {
  first: number;
  after: string | null;
};

export function useConnectionSpecPagination<
  TReadFromStore extends {
    parameters: UseConnectionSpecPaginationArgs;
    data: object;
  },
  TItem,
>(
  loadableField: LoadableField<
    TReadFromStore,
    Connection<TItem>,
    UseConnectionSpecPaginationArgs
  >,
  initialArgs?: {
    after?: string | void | null;
  },
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

      const firstParameter = {
        data: readOutDataAndRecords[i].item,
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

        return {
          fragmentReference,
          readerAst: readerWithRefetchQueries.readerArtifact.readerAst,
          records: readOutDataAndRecords[i],
          callback(_data) {
            rerender({});
          },
        };
      },
    );
  }

  const getFetchMore =
    (after: string | null) =>
    (count: number): void => {
      const loadedField = loadableField({
        after: after,
        first: count,
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

  const mostRecentItem: LoadedFragmentReference<
    TReadFromStore,
    Connection<TItem>
  > | null = loadedReferences[loadedReferences.length - 1];
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
      fetchMore: getFetchMore(initialArgs?.after ?? null),
      results: [],
      hasNextPage: true,
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
