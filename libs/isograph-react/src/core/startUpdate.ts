import type {
  ExtractData,
  ExtractStartUpdate,
  UnknownTReadFromStore,
} from './FragmentReference';
import type { IsographEnvironment } from './IsographEnvironment';

export function startUpdate<TReadFromStore extends UnknownTReadFromStore>(
  _environment: IsographEnvironment,
  _data: ExtractData<TReadFromStore>,
): ExtractStartUpdate<TReadFromStore> {
  return (_updater) => {};
}
