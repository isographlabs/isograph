import React from 'react';
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
  {} extends ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>
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
    fetchOptions?: FetchOptions<TResult, never>;
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

// @ts-ignore
function tsTests() {
  let neverArgs!: LoadableField<
    {
      parameters: Record<string, never>;
      data: {};
    },
    unknown
  >;

  let optionalArgs!: LoadableField<
    {
      parameters: {
        foo?: string;
      };
      data: {};
    },
    unknown
  >;

  let requiredArgs!: LoadableField<
    {
      parameters: {
        foo: string;
      };
      data: {};
    },
    unknown
  >;

  <LoadableFieldReader loadableField={neverArgs} children={() => {}} />;
  <LoadableFieldReader
    loadableField={neverArgs}
    children={() => {}}
    args={{}}
  />;
  <LoadableFieldReader
    loadableField={neverArgs}
    children={() => {}}
    args={{
      // @ts-expect-error
      foo: 'bar',
    }}
  />;

  <LoadableFieldReader loadableField={optionalArgs} children={() => {}} />;
  <LoadableFieldReader
    loadableField={optionalArgs}
    children={() => {}}
    args={{}}
  />;
  <LoadableFieldReader
    loadableField={optionalArgs}
    children={() => {}}
    args={{
      foo: 'bar',
    }}
  />;
  <LoadableFieldReader
    loadableField={optionalArgs}
    children={() => {}}
    args={{
      // @ts-expect-error
      foo: 12,
    }}
  />;

  // @ts-expect-error
  <LoadableFieldReader loadableField={requiredArgs} children={() => {}} />;
  <LoadableFieldReader
    loadableField={requiredArgs}
    children={() => {}}
    // @ts-expect-error
    args={{}}
  />;
  <LoadableFieldReader
    loadableField={requiredArgs}
    children={() => {}}
    args={{
      foo: 'bar',
    }}
  />;
  <LoadableFieldReader
    loadableField={requiredArgs}
    children={() => {}}
    args={{
      // @ts-expect-error
      foo: 12,
    }}
  />;
}
