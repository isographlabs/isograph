import type { ExtractData, ExtractStartUpdate } from './FragmentReference';
import type { IsographEnvironment } from './IsographEnvironment';
import type { StartUpdate } from './reader';

export function startUpdate<
  TReadFromStore extends {
    parameters: object;
    data: object;
    startUpdate?: StartUpdate<object>;
  },
>(
  _environment: IsographEnvironment,
  _data: ExtractData<TReadFromStore>,
): ExtractStartUpdate<TReadFromStore> {
  return (_updater) => {};
}
