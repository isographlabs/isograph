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

type FirstOrAfter = 'first' | 'after';
type OmitFirstAfter<TArgs> = keyof Omit<TArgs, FirstOrAfter> extends never
  ? void | Record<string, never>
  : Omit<TArgs, FirstOrAfter>;

type UsePaginationReturnValue<
  TReadFromStore extends { parameters: object; data: object },
  TItem,
  TArgs,
> =
  | {
      kind: 'Pending';
      pendingFragment: FragmentReference<TReadFromStore, ReadonlyArray<TItem>>;
      results: ReadonlyArray<TItem>;
    }
  | {
      kind: 'Complete';
      fetchMore: (args: OmitFirstAfter<TArgs>, first: number) => void;
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

function flatten<TItem>(
  arr: ReadonlyArray<Connection<TItem>>,
): NonNullConnection<TItem> {
  return arr.reduce<NonNullConnection<TItem>>(
    (acc, connection) => ({
      ...acc,
      ...connection,
      edges: acc.edges.concat(connection.edges ?? []),
    }),
    {
      edges: [],
      pageInfo: {
        hasNextPage: true,
        endCursor: null,
      },
    },
  );
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

export function usePagination<
  TArgs extends {
    first: number | void | null;
    after: string | void | null;
  },
  TItem,
  TReadFromStore extends { parameters: object; data: object },
>(
  loadableField: LoadableField<TArgs, Connection<TItem>>,
): UsePaginationReturnValue<TReadFromStore, TItem, TArgs> {
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
  ) {
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

    const items = flatten(results);
    return items;
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
    (after: string | null | undefined) =>
    (args: OmitFirstAfter<TArgs>, first: number): void => {
      // @ts-expect-error
      const loadedField = loadableField({
        ...args,
        after: after,
        first: first,
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
      fetchMore: getFetchMore(null),
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
        //@ts-expect-error map Connection<TItem> to ReadonlyArray<TItem>
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