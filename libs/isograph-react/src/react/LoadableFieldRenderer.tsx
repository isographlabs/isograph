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
> =
  {} extends ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>
    ? {
        args?: ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>;
      }
    : {
        args: ArgsWithoutProvidedArgs<TReadFromStore, TProvidedArgs>;
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
    fetchOptions?: FetchOptions<React.FC<TProps>, never>;
    networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
    additionalProps: Omit<TProps, keyof JSX.IntrinsicAttributes>;
  } & MaybeRequiredArgs<TReadFromStore, TProvidedArgs>,
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

// @ts-ignore
function tsTests() {
  let neverArgs!: LoadableField<
    {
      parameters: Record<string, never>;
      data: {};
    },
    () => React.ReactNode
  >;

  let optionalArgs!: LoadableField<
    {
      parameters: {
        foo?: string;
      };
      data: {};
    },
    () => React.ReactNode
  >;

  let requiredArgs!: LoadableField<
    {
      parameters: {
        foo: string;
      };
      data: {};
    },
    () => React.ReactNode
  >;

  <LoadableFieldRenderer loadableField={neverArgs} additionalProps={{}} />;
  <LoadableFieldRenderer
    loadableField={neverArgs}
    additionalProps={{}}
    args={{}}
  />;
  <LoadableFieldRenderer
    loadableField={neverArgs}
    additionalProps={{}}
    args={{
      // @ts-expect-error
      foo: 'bar',
    }}
  />;

  <LoadableFieldRenderer loadableField={optionalArgs} additionalProps={{}} />;
  <LoadableFieldRenderer
    loadableField={optionalArgs}
    additionalProps={{}}
    args={{}}
  />;
  <LoadableFieldRenderer
    loadableField={optionalArgs}
    additionalProps={{}}
    args={{
      foo: 'bar',
    }}
  />;
  <LoadableFieldRenderer
    loadableField={optionalArgs}
    additionalProps={{}}
    args={{
      // @ts-expect-error
      foo: 12,
    }}
  />;

  // @ts-expect-error
  <LoadableFieldRenderer loadableField={requiredArgs} additionalProps={{}} />;
  <LoadableFieldRenderer
    loadableField={requiredArgs}
    additionalProps={{}}
    // @ts-expect-error
    args={{}}
  />;
  <LoadableFieldRenderer
    loadableField={requiredArgs}
    additionalProps={{}}
    args={{
      foo: 'bar',
    }}
  />;
  <LoadableFieldRenderer
    loadableField={requiredArgs}
    additionalProps={{}}
    args={{
      // @ts-expect-error
      foo: 12,
    }}
  />;
}
