import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { getOrCreateItemInSuspenseCache } from '../core/cache';
import { FetchOptions } from '../core/check';
import {
  ExtractParameters,
  FragmentReference,
  type UnknownTReadFromStore,
} from '../core/FragmentReference';
import { LoadableField } from '../core/reader';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';

export function useClientSideDefer<
  TReadFromStore extends UnknownTReadFromStore,
  TResult,
>(
  loadableField: LoadableField<
    TReadFromStore,
    TResult,
    ExtractParameters<TReadFromStore>
  >,
  args?: Record<PropertyKey, never>,
  fetchOptions?: FetchOptions<TResult>,
): { fragmentReference: FragmentReference<TReadFromStore, TResult> };

export function useClientSideDefer<
  TReadFromStore extends UnknownTReadFromStore,
  TResult,
  TProvidedArgs extends object,
>(
  loadableField: LoadableField<
    TReadFromStore,
    TResult,
    Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>
  >,
  args: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>,
  fetchOptions?: FetchOptions<TResult>,
): { fragmentReference: FragmentReference<TReadFromStore, TResult> };

export function useClientSideDefer<
  TReadFromStore extends UnknownTReadFromStore,
  TResult,
  TProvidedArgs extends object,
>(
  loadableField: LoadableField<
    TReadFromStore,
    TResult,
    Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>
  >,
  args?: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>,
  fetchOptions?: FetchOptions<TResult>,
): { fragmentReference: FragmentReference<TReadFromStore, TResult> } {
  const [id, loader] = loadableField(args, fetchOptions ?? {});
  const environment = useIsographEnvironment();
  const cache = getOrCreateItemInSuspenseCache(environment, id, loader);

  const fragmentReference = useLazyDisposableState(cache).state;

  return { fragmentReference };
}
