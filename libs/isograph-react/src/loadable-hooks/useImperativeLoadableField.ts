import { FragmentReference } from '../core/FragmentReference';
import {
  UnassignedState,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { LoadableField } from '../core/reader';

type UseImperativeLoadableFieldReturn<TArgs, TResult> = {
  fragmentReference: FragmentReference<any, TResult> | UnassignedState;
  loadField: (args: TArgs) => void;
};

export function useImperativeLoadableField<TArgs, TResult>(
  loadableField: LoadableField<TArgs, TResult>,
): UseImperativeLoadableFieldReturn<TArgs, TResult> {
  const { state, setState } =
    useUpdatableDisposableState<FragmentReference<any, TResult>>();

  return {
    loadField: (args: TArgs) => {
      const [_id, loader] = loadableField(args);
      setState(loader());
    },
    fragmentReference: state,
  };
}
