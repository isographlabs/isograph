import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { useIsographEnvironment } from '../react/IsographEnvironmentProvider';
import { getOrCreateItemInSuspenseCache } from '../core/cache';
import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { LoadableField } from '../core/reader';

export function useClientSideDefer<
  TReadFromStore extends { data: object; parameters: object },
  TResult,
>(
  loadableField: LoadableField<
    TReadFromStore,
    TResult,
    ExtractParameters<TReadFromStore>
  >,
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
): { fragmentReference: FragmentReference<TReadFromStore, TResult> } {
  const [id, loader] = loadableField(args);
  const environment = useIsographEnvironment();
  const cache = getOrCreateItemInSuspenseCache(environment, id, loader);

  const fragmentReference = useLazyDisposableState(cache).state;

  return { fragmentReference };
}
