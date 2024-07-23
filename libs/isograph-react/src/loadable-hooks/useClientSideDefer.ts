import { FragmentReference } from '../FragmentReference';
import { useIsographEnvironment } from '../IsographEnvironmentProvider';
import { getOrCreateCache } from '../cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { LoadableField } from '../reader';

export function useClientSideDefer<TResult>(
  loadableField: LoadableField<void, TResult>,
): FragmentReference<any, TResult>;

export function useClientSideDefer<TArgs, TResult>(
  loadableField: LoadableField<TArgs, TResult>,
  args: TArgs,
): FragmentReference<any, TResult>;

// TODO allow the user to pass props somehow
export function useClientSideDefer<TArgs, TResult>(
  loadableField: LoadableField<TArgs, TResult>,
  args?: TArgs,
): FragmentReference<any, TResult> {
  // @ts-expect-error args is missing iff it has the type void
  const [id, loader] = loadableField(args);
  const environment = useIsographEnvironment();
  const cache = getOrCreateCache(environment, id, loader);

  const fragmentReference = useLazyDisposableState(cache).state;

  return fragmentReference;
}
