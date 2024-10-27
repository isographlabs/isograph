import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { getOrCreateItemInSuspenseCache } from '../core/cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { LoadableField } from '../core/reader';
import { FetchOptions } from '../core/check';

export function useClientSideDefer<
  TReadFromStore extends { data: object; parameters: object },
  TResult,
>(
  loadableField: LoadableField<
    TReadFromStore,
    TResult,
    ExtractParameters<TReadFromStore>
  >,
  fetchOptions?: FetchOptions,
): { fragmentReference: FragmentReference<TReadFromStore, TResult> };

export function useClientSideDefer<
  TReadFromStore extends { data: object; parameters: object },
  TResult,
  TProvidedArgs extends object,
>(
  loadableField: LoadableField<
    TReadFromStore,
    TResult,
    Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>
  >,
  args: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>,
  fetchOptions?: FetchOptions,
): { fragmentReference: FragmentReference<TReadFromStore, TResult> };

export function useClientSideDefer<
  TReadFromStore extends { data: object; parameters: object },
  TResult,
  TProvidedArgs extends object,
>(
  loadableField: LoadableField<
    TReadFromStore,
    TResult,
    Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>
  >,
  args?: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>,
  fetchOptions?: FetchOptions,
): { fragmentReference: FragmentReference<TReadFromStore, TResult> } {
  const [id, loader] = loadableField(args, fetchOptions ?? {});
  const environment = useIsographEnvironment();
  const cache = getOrCreateItemInSuspenseCache(environment, id, loader);

  const fragmentReference = useLazyDisposableState(cache).state;

  return { fragmentReference };
}
