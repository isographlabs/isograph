import { type FetchOptions } from '../core/check';
import {
  ExtractParameters,
  type UnknownTReadFromStore,
} from '../core/FragmentReference';
import { type NetworkRequestReaderOptions } from '../core/read';
import { type LoadableField } from '../core/reader';
import { useClientSideDefer } from '../loadable-hooks/useClientSideDefer';
import { useResult } from './useResult';

export function LoadableFieldReader<
  TReadFromStore extends UnknownTReadFromStore,
  TResult,
  TProvidedArgs extends object,
  TChildrenResult,
>(props: {
  loadableField: LoadableField<
    TReadFromStore,
    TResult,
    Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>
  >;
  // TODO we can improve this to not require args if its an empty object
  args: Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>;
  fetchOptions?: FetchOptions<TResult>;
  networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
  children: (arg: TResult) => TChildrenResult;
}): TChildrenResult {
  const { fragmentReference } = useClientSideDefer(
    props.loadableField,
    props.args,
    props.fetchOptions,
  );

  const readOutFragmentData = useResult(
    fragmentReference,
    props.networkRequestOptions,
  );

  return props.children(readOutFragmentData);
}
