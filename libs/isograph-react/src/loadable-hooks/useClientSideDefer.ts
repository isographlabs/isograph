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

type ArgsWithoutProvidedArgs<
  TReadFromStore extends UnknownTReadFromStore,
  TProvidedArgs extends object,
> = Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>;

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
  ...maybeRequiredArgs: {} extends ArgsWithoutProvidedArgs<
    TReadFromStore,
    TProvidedArgs
  >
    ? [
        args?: ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>,
        fetchOptions?: FetchOptions<TResult>,
      ]
    : [
        args: ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>,
        fetchOptions?: FetchOptions<TResult>,
      ]
): { fragmentReference: FragmentReference<TReadFromStore, TResult> } {
  const [args, fetchOptions] = maybeRequiredArgs;

  const [id, loader] = loadableField(args, fetchOptions ?? {});
  const environment = useIsographEnvironment();
  const cache = getOrCreateItemInSuspenseCache(environment, id, loader);

  const fragmentReference = useLazyDisposableState(cache).state;

  return { fragmentReference };
}

// @ts-ignore
function tsTests() {
  let optionalArgs!: LoadableField<
    {
      parameters: {
        foo?: string;
      };
      data: {};
    },
    unknown
  >;

  let requiredArgs!: LoadableField<
    {
      parameters: {
        foo: string;
      };
      data: {};
    },
    unknown
  >;

  useClientSideDefer(optionalArgs);
  useClientSideDefer(optionalArgs, {});
  useClientSideDefer(optionalArgs, {
    foo: 'bar',
  });
  useClientSideDefer(optionalArgs, {
    // @ts-expect-error
    foo: 12,
  });

  // @ts-expect-error
  useClientSideDefer(requiredArgs);
  // @ts-expect-error
  useClientSideDefer(requiredArgs, {});
  useClientSideDefer(requiredArgs, {
    foo: 'bar',
  });
  useClientSideDefer(requiredArgs, {
    // @ts-expect-error
    foo: 12,
  });
}
