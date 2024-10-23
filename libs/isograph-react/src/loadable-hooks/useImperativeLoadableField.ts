import {
  ExtractParameters,
  FragmentReference,
} from '../core/FragmentReference';
import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { LoadableField } from '../core/reader';

type UseImperativeLoadableFieldReturn<
  TReadFromStore extends { data: object; parameters: object },
  TResult,
  TProvidedArgs extends object,
> = {
  fragmentReference:
    | FragmentReference<TReadFromStore, TResult>
    | UnassignedState;
  loadField: (
    args: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs> | void,
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
    ) => {
      const [_id, loader] = loadableField(args);
      setState(loader());
    },
    fragmentReference: state,
  };
}
