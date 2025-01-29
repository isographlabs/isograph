import { useLazyDisposableState } from '@isograph/react-disposable-state';
import { getOrCreateCacheForArtifact } from '../core/cache';
import { FetchOptions, type RequiredFetchOptions } from '../core/check';
import {
  IsographEntrypoint,
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { logMessage } from '../core/logging';
import type { StartUpdate } from '../core/reader';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

type IsExactlyEqual<TLeft, TRight, TIfExactlyEqual, TIfNot> =
  TLeft extends TRight
    ? TRight extends TLeft
      ? TIfExactlyEqual
      : TIfNot
    : TIfNot;

export function useLazyReference<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    TNormalizationAst
  >,
  variables: ExtractParameters<TReadFromStore>,

  // What is going on here? If TNormalizationAst is exactly NormalizationAst,
  // then (and only then) we can pass an optional FetchOptions.
  //
  // In all other cases, we must pass RequiredFetchOptions, and it must
  // be present.
  // ...[fetchOptions]: TNormalizationAst extends {
  //   kind: 'NormalizationAstLoader';
  // }
  //   ? [fetchOptions: RequiredFetchOptions<TClientFieldValue>]
  //   : [fetchOptions?: FetchOptions<TClientFieldValue>]
  // ...[fetchOptions]: Equal2<
  //   TFetchPolicy,
  //   true,
  //   [fetchOptions?: FetchOptions<TClientFieldValue>],
  //   [fetchOptions: RequiredFetchOptions<TClientFieldValue>]
  // >
  // TFetchPolicy extends true
  //   ? [fetchOptions?: FetchOptions<TClientFieldValue>]
  //   : [fetchOptions: RequiredFetchOptions<TClientFieldValue>]
  // ...[fetchOptions]: Equal2<
  //   TNormalizationAst,
  //   NormalizationAst,
  //   [fetchOptions?: FetchOptions<TClientFieldValue>],
  //   [fetchOptions: RequiredFetchOptions<TClientFieldValue>]
  // >
  ...[fetchOptions]: Equal2<
    TNormalizationAst,
    NormalizationAstLoader,
    [fetchOptions: RequiredFetchOptions<TClientFieldValue>],
    [fetchOptions?: FetchOptions<TClientFieldValue>]
  >
): // TNormalizationAst extends NormalizationAst | NormalizationAstLoader
//   ? unknown
//   : {
//       fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>;
//     } {
Equal2<
  TNormalizationAst,
  NormalizationAst | NormalizationAstLoader,
  unknown,
  {
    fragmentReference: FragmentReference<TReadFromStore, TClientFieldValue>;
  }
> {
  const environment = useIsographEnvironment();

  if (entrypoint?.kind !== 'Entrypoint') {
    // TODO have a separate error logger
    logMessage(environment, {
      kind: 'NonEntrypointReceived',
      entrypoint,
    });
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

export type Equal2<X, Y, IfTrue, IfFalse> =
  (<T>() => T extends X ? 1 : 2) extends <T>() => T extends Y ? 1 : 2
    ? IfTrue
    : IfFalse;

// @ts-ignore
function tsTests() {
  let withAst!: IsographEntrypoint<any, unknown, NormalizationAst>;
  let withAstLoader!: IsographEntrypoint<any, unknown, NormalizationAstLoader>;
  let withAstOrLoader!: IsographEntrypoint<
    any,
    unknown,
    NormalizationAst | NormalizationAstLoader
  >;

  useLazyReference(withAst, {});
  useLazyReference(withAst, {}, { shouldFetch: 'Yes' });
  useLazyReference(withAst, {}, { shouldFetch: 'IfNecessary' });

  // @ts-expect-error if there's no ast, require `shouldFetch` to be specified
  useLazyReference(withAstLoader, {});
  useLazyReference(withAstLoader, {}, { shouldFetch: 'Yes' });
  // @ts-expect-error if there's no ast, `shouldFetch` can't be `IfNecessary`
  useLazyReference(withAstLoader, {}, { shouldFetch: 'IfNecessary' });

  // These ts-expect-errors do not fire:
  // if the type is unknown there can be no ast so we should use the same rules
  useLazyReference(withAstOrLoader, {});
  useLazyReference(withAstOrLoader, {}, { shouldFetch: 'Yes' });
  useLazyReference(withAstOrLoader, {}, { shouldFetch: 'IfNecessary' });
}

// @ts-ignore
type Blah = IsExactlyEqual<
  NormalizationAst | NormalizationAstLoader,
  NormalizationAst,
  'They are equal?',
  'Of course theyre not'
>;
// @ts-ignore
type Blah2 = IsExactlyEqual<
  NormalizationAst,
  NormalizationAst | NormalizationAstLoader,
  'They are equal?',
  'Of course theyre not'
>;

// @ts-ignore
type Blah3 = Equal2<
  NormalizationAst,
  NormalizationAst | NormalizationAstLoader,
  'they are equal',
  'not this time bucko'
>;
