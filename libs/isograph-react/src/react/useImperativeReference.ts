import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import type { NetworkResponseObject } from '../core/cache';
import { FetchOptions, type RequiredFetchOptions } from '../core/check';
import {
  IsographEntrypoint,
  type NormalizationAst,
  type NormalizationAstLoader,
} from '../core/entrypoint';
import {
  ExtractParameters,
  FragmentReference,
  type UnknownTReadFromStore,
} from '../core/FragmentReference';
import {
  getOrLoadReaderWithRefetchQueries,
  ROOT_ID,
} from '../core/IsographEnvironment';
import { maybeMakeNetworkRequest } from '../core/makeNetworkRequest';
import { useIsographEnvironment } from './IsographEnvironmentProvider';

export type UseImperativeReferenceResult<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
  TRawResponseType,
> = {
  fragmentReference: FragmentReference<
    TReadFromStore,
    TClientFieldValue
  > | null;
  loadFragmentReference: (
    variables: ExtractParameters<TReadFromStore>,
    ...[fetchOptions]: NormalizationAstLoader extends TNormalizationAst
      ? [
          fetchOptions: RequiredFetchOptions<
            TClientFieldValue,
            TRawResponseType
          >,
        ]
      : [fetchOptions?: FetchOptions<TClientFieldValue, TRawResponseType>]
  ) => void;
};

export function useImperativeReference<
  TReadFromStore extends UnknownTReadFromStore,
  TClientFieldValue,
  TNormalizationAst extends NormalizationAst | NormalizationAstLoader,
  TRawResponseType extends NetworkResponseObject,
>(
  entrypoint: IsographEntrypoint<
    TReadFromStore,
    TClientFieldValue,
    TNormalizationAst,
    TRawResponseType
  >,
): UseImperativeReferenceResult<
  TReadFromStore,
  TClientFieldValue,
  TNormalizationAst,
  TRawResponseType
> {
  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReference<TReadFromStore, TClientFieldValue>
    >();
  const environment = useIsographEnvironment();
  return {
    fragmentReference: state !== UNASSIGNED_STATE ? state : null,
    loadFragmentReference: (
      variables: ExtractParameters<TReadFromStore>,
      fetchOptions?: FetchOptions<TClientFieldValue, TRawResponseType>,
    ) => {
      const { fieldName, readerArtifactKind, readerWithRefetchQueries } =
        getOrLoadReaderWithRefetchQueries(
          environment,
          entrypoint.readerWithRefetchQueries,
        );
      const [networkRequest, disposeNetworkRequest] = maybeMakeNetworkRequest(
        environment,
        entrypoint,
        variables,
        readerWithRefetchQueries,
        fetchOptions ?? null,
      );
      setState([
        {
          kind: 'FragmentReference',
          readerWithRefetchQueries,
          fieldName,
          readerArtifactKind,
          root: { __link: ROOT_ID, __typename: entrypoint.concreteType },
          variables,
          networkRequest,
        },
        () => {
          disposeNetworkRequest();
        },
      ]);
    },
  };
}
