import type { ReaderWithRefetchQueries } from './entrypoint';
import {
  stableIdForFragmentReference,
  type ExtractStartUpdate,
  type FragmentReference,
  type UnknownTReadFromStore,
} from './FragmentReference';
import type { IsographEnvironment } from './IsographEnvironment';

export function getOrCreateCachedStartUpdate<
  TReadFromStore extends UnknownTReadFromStore,
>(
  environment: IsographEnvironment,
  fragmentReference: FragmentReference<any, any>,
  readerWithRefetchQueries: ReaderWithRefetchQueries<any, any>,
): ExtractStartUpdate<TReadFromStore> {
  const cachedStartUpdateByResolver = environment.startUpdateCache;

  let startUpdateById = cachedStartUpdateByResolver.get(
    readerWithRefetchQueries.readerArtifact.resolver,
  );

  if (startUpdateById === undefined) {
    startUpdateById = {};
    cachedStartUpdateByResolver.set(
      readerWithRefetchQueries.readerArtifact.resolver,
      startUpdateById,
    );
  }

  return (startUpdateById[stableIdForFragmentReference(fragmentReference)] ??=
    (() => {
      let startUpdate: ExtractStartUpdate<TReadFromStore> | undefined = (
        _updater,
      ) => {
        // TODO start update
      };

      return startUpdate;
    })());
}
