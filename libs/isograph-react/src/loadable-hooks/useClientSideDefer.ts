import { FragmentReference } from '../core/FragmentReference';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { getOrCreateItemInSuspenseCache } from '../core/cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { LoadableField } from '../core/reader';

export function useClientSideDefer<TResult>(
  loadableField: LoadableField<void, TResult>,
): { fragmentReference: FragmentReference<Record<string, never>, TResult> };

export function useClientSideDefer<TArgs extends Object, TResult>(
  loadableField: LoadableField<TArgs, TResult>,
  args: TArgs,
): { fragmentReference: FragmentReference<TArgs, TResult> };

export function useClientSideDefer<TArgs extends Object, TResult>(
  loadableField: LoadableField<TArgs, TResult>,
  args?: TArgs,
): { fragmentReference: FragmentReference<TArgs, TResult> } {
  // @ts-expect-error args is missing iff it has the type void
  const [id, loader] = loadableField(args);
  const environment = useIsographEnvironment();
  const cache = getOrCreateItemInSuspenseCache(environment, id, loader);

  const fragmentReference = useLazyDisposableState(cache).state;

  return { fragmentReference };
}
