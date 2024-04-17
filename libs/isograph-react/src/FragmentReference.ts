import { DataId } from './IsographEnvironment';
import { RefetchQueryArtifactWrapper } from './entrypoint';
import { ReaderArtifact } from './reader';

// TODO type this better
export type Variable = any;

export type Variables = { readonly [index: string]: Variable };

export type FragmentReference<
  TReadFromStore extends Object,
  TClientFieldValue,
> = {
  kind: 'FragmentReference';
  readerArtifact: ReaderArtifact<TReadFromStore, TClientFieldValue>;
  root: DataId;
  variables: Variables | null;
  // TODO: We should instead have ReaderAst<TClientFieldProps>
  nestedRefetchQueries: RefetchQueryArtifactWrapper[];
};
