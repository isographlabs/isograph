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

type SkipOrLimit = 'skip' | 'limit';
type OmitSkipLimit<TArgs> = keyof Omit<TArgs, SkipOrLimit> extends never
  ? void | Record<string, never>
  : Omit<TArgs, SkipOrLimit>;

type UseSkipLimitReturnValue<TArgs, TItem> = {
  readonly fetchMore: (args: OmitSkipLimit<TArgs>, count: number) => void;
  readonly results: ReadonlyArray<TItem>;
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
 */
export function useSuspensefulSkipLimitPagination<
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

  const loadedReferences = state === UNASSIGNED_STATE ? [] : state;

  const results = loadedReferences.map(([pointer]) => {
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
    return fragmentReference.readerWithRefetchQueries.readerArtifact.resolver(
      data.item,
      undefined,
    ) as ReadonlyArray<any>;
  });

  const items = flatten(results);
  const loadedSoFar = items.length;

  return {
    fetchMore: (args, count) => {
      // @ts-expect-error
      const loadedField = loadableField({
        ...args,
        skip: loadedSoFar,
        limit: count,
      })[1]();
      const newPointer = createReferenceCountedPointer(loadedField);
      const clonedPointers = [
        ...loadedReferences.map(([refCountedPointer]) => {
          const clonedRefCountedPointer =
            refCountedPointer.cloneIfNotDisposed();
          if (clonedRefCountedPointer == null) {
            throw new Error(
              'This reference counted pointer has already been disposed. \
              This is indicative of a bug in useSuspensefulSkipLimitPagination.',
            );
          }
          return clonedRefCountedPointer;
        }),
      ];

      const allPointers = [...clonedPointers, newPointer];

      const totalItemCleanupPair: ItemCleanupPair<
        ReadonlyArray<
          ItemCleanupPair<
            ReferenceCountedPointer<ArrayFragmentReference<TItem>>
          >
        >
      > = [
        allPointers,
        () => {
          allPointers.forEach(([, dispose]) => {
            dispose();
          });
        },
      ];

      setState(totalItemCleanupPair);
    },
    results: items,
  };
}
