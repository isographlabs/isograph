import { DataId } from './IsographEnvironment';
import { RefetchQueryNormalizationArtifactWrapper } from './entrypoint';
import { ReaderArtifact } from './reader';

// TODO type this better
export type Variable = any;

export type Variables = { readonly [index: string]: Variable };

export type FragmentReference<
  TReadFromStore extends Object,
  TClientFieldValue,
> = {
  readonly kind: 'FragmentReference';
  readonly readerArtifact: ReaderArtifact<TReadFromStore, TClientFieldValue>;
  readonly root: DataId;
  readonly variables: Variables | null;
  // TODO: We should instead have ReaderAst<TClientFieldProps>
  readonly nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[];
};
