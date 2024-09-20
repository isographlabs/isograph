import { LoadableField } from '../core/reader';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { ItemCleanupPair } from '@isograph/disposable-types';
import { FragmentReference } from '../core/FragmentReference';
import { maybeUnwrapNetworkRequest } from '../react/useResult';
import { readButDoNotEvaluate } from '../core/read';
import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import {
  createReferenceCountedPointer,
  ReferenceCountedPointer,
} from '@isograph/reference-counted-pointer';
import { getPromiseState, readPromise } from '../core/PromiseWrapper';

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

type LoadedFragmentReferences<TItem> = ReadonlyArray<
  ItemCleanupPair<ReferenceCountedPointer<ArrayFragmentReference<TItem>>>
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

/**
 * accepts a loadableField that accepts skip and limit arguments
 * and returns:
 * - a fetchMore function that, when called, triggers a network
 *   request for additional data, and
 * - the data received so far.
 *
 * This hook will suspend if any network request is in flight.
 *
 * Calling fetchMore before the hook mounts is a no-op.
 *
 * NOTE: this hook does not subscribe to changes. This is a known
 * issue. If you are running into this issue, reach out on GitHub/
 * Twitter, and we'll fix the issue.
 */
export function useSkipLimitPagination<
  TArgs extends {
    skip: number | void | null;
    limit: number | void | null;
  },
  TItem,
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
  function readCompletedFragmentReferences(
    completedReferences: ReadonlyArray<
      ItemCleanupPair<ReferenceCountedPointer<ArrayFragmentReference<TItem>>>
    >,
  ) {
    // In general, this will not suspend. But it could, if there is missing data.
    // A better version of this hook would not do any reading here.
    const results = completedReferences.map(([pointer]) => {
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

      return readerWithRefetchQueries.readerArtifact.resolver(
        data.item,
        undefined,
      ) as ReadonlyArray<any>;
    });

    const items = flatten(results);
    return items;
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
  if (loadedReferences.length === 0) {
    return {
      kind: 'Complete',
      fetchMore: getFetchMore(0),
      results: [],
    };
  }

  const mostRecentItem = loadedReferences[loadedReferences.length - 1];
  const mostRecentFragmentReference = mostRecentItem[0].getItemIfNotDisposed();
  if (mostRecentFragmentReference === null) {
    throw new Error(
      'FragmentReference is unexpectedly disposed. \
        This is indicative of a bug in Isograph.',
    );
  }

  const networkRequestStatus = getPromiseState(
    mostRecentFragmentReference.networkRequest,
  );
  switch (networkRequestStatus.kind) {
    case 'Pending': {
      const completedFragmentReferences = loadedReferences.slice(
        0,
        loadedReferences.length - 1,
      );
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
      const results = readCompletedFragmentReferences(loadedReferences);
      return {
        kind: 'Complete',
        results,
        fetchMore: getFetchMore(results.length),
      };
    }
  }
}
