import { DataId } from './IsographEnvironment';
import { RefetchQueryArtifactWrapper } from './entrypoint';
import { ReaderArtifact } from './reader';

// TODO type this better
export type Variable = any;

export type FragmentReference<
  TReadFromStore extends Object,
  TClientFieldValue,
> = {
  kind: 'FragmentReference';
  readerArtifact: ReaderArtifact<TReadFromStore, TClientFieldValue>;
  root: DataId;
  variables: { [index: string]: Variable } | null;
  // TODO: We should instead have ReaderAst<TClientFieldProps>
  nestedRefetchQueries: RefetchQueryArtifactWrapper[];
};
