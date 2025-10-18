import * as React from 'react';
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
  TResult,
  TChildrenResult,
  TProps,
> =
  ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs> extends Record<
    PropertyKey,
    never
  >
    ? {
        args?: ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>;
        fetchOptions?: FetchOptions<TResult>;
        networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
        children?: (arg: TResult) => TChildrenResult;
        additionalProps: Omit<TProps, keyof JSX.IntrinsicAttributes>;
      }
    : {
        args: ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>;
        fetchOptions?: FetchOptions<TResult>;
        networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
        children?: (arg: TResult) => TChildrenResult;
        additionalProps: Omit<TProps, keyof JSX.IntrinsicAttributes>;
      };

export function LoadableFieldRenderer<
  TReadFromStore extends UnknownTReadFromStore,
  TProvidedArgs extends object,
  TChildrenResult,
  TProps,
>(
  props: {
    loadableField: LoadableField<
      TReadFromStore,
      React.FC<TProps>,
      Omit<ExtractParameters<TReadFromStore>, keyof TProvidedArgs>
    >;
  } & MaybeRequiredArgs<
    TReadFromStore,
    TProvidedArgs,
    React.FC<TProps>,
    TChildrenResult,
    TProps
  >,
): TChildrenResult {
  const { fragmentReference } = useClientSideDefer(
    props.loadableField,
    // @ts-expect-error
    props.args,
    props.fetchOptions,
  );

  const Component = useResult(fragmentReference, props.networkRequestOptions);

  // TODO we probably can figure out a way to convince TypeScript of
  // the validity of this.
  // @ts-expect-error
  return <Component {...props.additionalProps} />;
}
