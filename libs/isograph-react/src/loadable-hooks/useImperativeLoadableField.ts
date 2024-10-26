import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { LoadableField } from '../core/reader';
import { FetchOptions } from '../core/check';

type UseImperativeLoadableFieldReturn<
  TReadFromStore extends { data: object; parameters: object },
  TResult,
  TProvidedArgs extends object,
> = {
  fragmentReference:
    | FragmentReference<TReadFromStore, TResult>
    | UnassignedState;
  loadField: (
    // TODO this should be void iff all args are provided by the query, like in
    // useClientSideDefer.
    args: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs> | void,
    fetchOptions?: FetchOptions,
  ) => void;
};

export function useImperativeLoadableField<
  TReadFromStore extends { data: object; parameters: object },
  TResult,
  TProvidedArgs extends object,
>(
  loadableField: LoadableField<
    TReadFromStore,
    TResult,
    Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>
  >,
): UseImperativeLoadableFieldReturn<TReadFromStore, TResult, TProvidedArgs> {
  const { state, setState } =
    useUpdatableDisposableState<FragmentReference<TReadFromStore, TResult>>();

  return {
    loadField: (
      args: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs> | void,
      fetchOptions?: FetchOptions,
    ) => {
      const [_id, loader] = loadableField(args, fetchOptions ?? {});
      setState(loader());
    },
    fragmentReference: state,
  };
}
