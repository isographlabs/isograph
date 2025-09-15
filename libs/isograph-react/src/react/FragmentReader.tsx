import {
  type ExtractReadFromStore,
  type IsographEntrypoint,
} from '../core/entrypoint';
import { type FragmentReference } from '../core/FragmentReference';
import { type NetworkRequestReaderOptions } from '../core/read';
import { useResult } from './useResult';

export function FragmentReader<
  TResult,
  TEntrypoint extends IsographEntrypoint<any, TResult, any>,
  TChildrenResult,
>({
  fragmentReference,
  networkRequestOptions,
  children,
}: {
  fragmentReference: FragmentReference<
    ExtractReadFromStore<TEntrypoint>,
    TResult
  >;
  networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
  children: (data: TResult) => TChildrenResult;
}): TChildrenResult {
  const result = useResult(fragmentReference, networkRequestOptions);
  return children(result);
}
