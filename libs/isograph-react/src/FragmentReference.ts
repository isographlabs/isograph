import { DataId } from './IsographEnvironment';
import { RefetchQueryArtifactWrapper } from './entrypoint';
import { ReaderArtifact } from './reader';

// TODO type this better
export type Variable = any;

export type FragmentReference<
  TReadFromStore extends Object,
  TResolverResult,
> = {
  kind: 'FragmentReference';
  readerArtifact: ReaderArtifact<TReadFromStore, TResolverResult>;
  root: DataId;
  variables: { [index: string]: Variable } | null;
  // TODO: We should instead have ReaderAst<TResolverProps>
  nestedRefetchQueries: RefetchQueryArtifactWrapper[];
};
