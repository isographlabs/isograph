import { DataId } from './IsographEnvironment';
import { RefetchQueryNormalizationArtifactWrapper } from './entrypoint';
import { TopLevelReaderArtifact } from './reader';

// TODO type this better
export type Variable = any;

export type Variables = { readonly [index: string]: Variable };

export type FragmentReference<
  TReadFromStore extends Object,
  TClientFieldValue,
> = {
  readonly kind: 'FragmentReference';
  readonly readerArtifact: TopLevelReaderArtifact<
    TReadFromStore,
    TClientFieldValue,
    any
  >;
  readonly root: DataId;
  readonly variables: Variables | null;
  // TODO: We should instead have ReaderAst<TClientFieldProps>
  readonly nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[];
};
