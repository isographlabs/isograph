import { DataId } from './IsographEnvironment';
import { RefetchQueryNormalizationArtifactWrapper } from '../core/entrypoint';
import { TopLevelReaderArtifact } from './reader';
import { PromiseWrapper } from './PromiseWrapper';

// TODO type this better
export type VariableValue = string | number | boolean | null | object;

export type Variables = { readonly [index: string]: VariableValue };

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
  readonly networkRequest: PromiseWrapper<void, any>;
};
