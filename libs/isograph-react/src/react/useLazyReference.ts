import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { getOrCreateCacheForArtifact } from '../core/cache';
import { FetchOptions, type RequiredFetchOptions } from '../core/check';
import {
  IsographEntrypoint,
  type ExtractNormalizationAst,
  type ExtractReadFromStore,
  type ExtractResolverResult,
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { logMessage } from '../core/logging';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

export function useLazyReference<
  TEntrypoint extends
    | IsographEntrypoint<any, any, NormalizationAst>
    | IsographEntrypoint<any, any, NormalizationAstLoader>,
>(
  entrypoint: TEntrypoint,
  variables: ExtractParameters<ExtractReadFromStore<TEntrypoint>>,
  ...[
    fetchOptions,
  ]: NormalizationAstLoader extends ExtractNormalizationAst<TEntrypoint>
    ? [fetchOptions: RequiredFetchOptions<ExtractResolverResult<TEntrypoint>>]
    : [fetchOptions?: FetchOptions<ExtractResolverResult<TEntrypoint>>]
): {
  fragmentReference: FragmentReference<
    ExtractReadFromStore<TEntrypoint>,
    ExtractResolverResult<TEntrypoint>
  >;
} {
  const environment = useIsographEnvironment();

  if (entrypoint?.kind !== 'Entrypoint') {
    // TODO have a separate error logger
    logMessage(environment, () => ({
      kind: 'NonEntrypointReceived',
      entrypoint,
    }));
  }

  const cache = getOrCreateCacheForArtifact(
    environment,
    entrypoint,
    variables,
    fetchOptions,
  );

  return {
    fragmentReference: useLazyDisposableState(cache).state,
  };
}

// @ts-ignore
function tsTests() {
  let withAst!: IsographEntrypoint<any, any, NormalizationAst>;
  let withAstLoader!: IsographEntrypoint<any, any, NormalizationAstLoader>;
  let withAstOrLoader = Math.random() ? withAst : withAstLoader;

  useLazyReference(withAst, {}) satisfies {};
  useLazyReference(withAst, {}, { shouldFetch: 'Yes' }) satisfies {};
  useLazyReference(withAst, {}, { shouldFetch: 'IfNecessary' }) satisfies {};

  // @ts-expect-error if there's no ast, require `shouldFetch` to be specified
  useLazyReference(withAstLoader, {});
  useLazyReference(withAstLoader, {}, { shouldFetch: 'Yes' }) satisfies {};
  // @ts-expect-error if there's no ast, `shouldFetch` can't be `IfNecessary`
  useLazyReference(withAstLoader, {}, { shouldFetch: 'IfNecessary' });

  // if the type is unknown there can be no ast so we should use the same rules
  // @ts-expect-error if there's no ast, require `shouldFetch` to be specified
  useLazyReference(withAstOrLoader, {});
  useLazyReference(withAstOrLoader, {}, { shouldFetch: 'Yes' }) satisfies {};
  // @ts-expect-error if there's no ast, `shouldFetch` can't be `IfNecessary`
  useLazyReference(withAstOrLoader, {}, { shouldFetch: 'IfNecessary' });

  type Foo = {
    parameters: { foo: '' };
    data: any;
  };
  type Bar = {
    parameters: { bar: '' };
    data: any;
  };
  type Baz = {
    parameters: { baz: '' };
    data: any;
  };

  let withVariables!:
    | IsographEntrypoint<Foo, 'Foo', NormalizationAst>
    | IsographEntrypoint<Bar, 'Bar', NormalizationAstLoader>
    | IsographEntrypoint<Baz, 'Baz', NormalizationAstLoader>;

  useLazyReference(
    withVariables,
    {
      foo: '',
      bar: '',
      baz: '',
    },
    { shouldFetch: 'Yes' },
  ) satisfies {
    readonly fragmentReference: FragmentReference<any, 'Foo' | 'Bar' | 'Baz'>;
  };
}
