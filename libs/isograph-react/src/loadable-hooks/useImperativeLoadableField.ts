import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { FetchOptions } from '../core/check';
import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import { LoadableField } from '../core/reader';

export type UseImperativeLoadableFieldReturn<
  TReadFromStore extends { data: object; parameters: object },
  TResult,
  TProvidedArgs extends object,
> = {
  fragmentReference: FragmentReference<TReadFromStore, TResult> | null;
  loadFragmentReference: (
    // TODO this should be void iff all args are provided by the query, like in
    // useClientSideDefer.
    args: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs> | void,
    fetchOptions?: FetchOptions<TResult>,
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
    loadFragmentReference: (
      args: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs> | void,
      fetchOptions?: FetchOptions<TResult>,
    ) => {
      const [_id, loader] = loadableField(args, fetchOptions ?? {});
      setState(loader());
    },
    fragmentReference: state !== UNASSIGNED_STATE ? state : null,
  };
}
