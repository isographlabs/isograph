import { type FetchOptions } from '../core/check';
import {
  ExtractParameters,
  type UnknownTReadFromStore,
} from '../core/FragmentReference';
import { type NetworkRequestReaderOptions } from '../core/read';
import { type LoadableField } from '../core/reader';
import { useClientSideDefer } from '../loadable-hooks/useClientSideDefer';
import { useResult } from './useResult';

type ArgsWithoutProvidedArgs<
  TReadFromStore extends UnknownTReadFromStore,
  TProvidedArgs extends object,
> = Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>;

type MaybeRequiredArgs<
  TReadFromStore extends UnknownTReadFromStore,
  TProvidedArgs extends object,
> =
  ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs> extends Record<
    PropertyKey,
    never
  >
    ? {
        args?: ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>;
      }
    : {
        args: ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>;
      };

export function LoadableFieldReader<
  TReadFromStore extends UnknownTReadFromStore,
  TResult,
  TProvidedArgs extends object,
  TChildrenResult,
>(
  props: {
    loadableField: LoadableField<
      TReadFromStore,
      TResult,
      Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>
    >;
    fetchOptions?: FetchOptions<TResult>;
    networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
    children: (arg: TResult) => TChildrenResult;
  } & MaybeRequiredArgs<TReadFromStore, TProvidedArgs>,
): TChildrenResult {
  const { fragmentReference } = useClientSideDefer(
    props.loadableField,
    // @ts-expect-error
    props.args,
    props.fetchOptions,
  );

  const readOutFragmentData = useResult(
    fragmentReference,
    props.networkRequestOptions,
  );

  return props.children(readOutFragmentData);
}
