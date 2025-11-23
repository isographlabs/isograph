import * as React from 'react';
import {
  type ExtractReadFromStore,
  type IsographEntrypoint,
} from '../core/entrypoint';
import { type FragmentReference } from '../core/FragmentReference';
import { type NetworkRequestReaderOptions } from '../core/read';
import { useResult } from './useResult';
import type { NetworkResponseObject } from '../core/cache';

export type IsExactlyIntrinsicAttributes<T> = T extends JSX.IntrinsicAttributes
  ? JSX.IntrinsicAttributes extends T
    ? true
    : false
  : false;

export function FragmentRenderer<
  TProps extends Record<any, any>,
  TRawResponseType extends NetworkResponseObject,
  TEntrypoint extends IsographEntrypoint<
    any,
    React.FC<TProps>,
    any,
    TRawResponseType
  >,
>(
  props: IsExactlyIntrinsicAttributes<TProps> extends true
    ? {
        fragmentReference: FragmentReference<
          ExtractReadFromStore<TEntrypoint>,
          React.FC<TProps>
        >;
        additionalProps?: Record<PropertyKey, never>;
        networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
      }
    : {
        fragmentReference: FragmentReference<
          ExtractReadFromStore<TEntrypoint>,
          React.FC<TProps>
        >;
        additionalProps: Omit<TProps, keyof JSX.IntrinsicAttributes>;
        networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
      },
): React.ReactNode {
  const Component = useResult(
    props.fragmentReference,
    props.networkRequestOptions,
  );
  // TypeScript is not understanding that if additionalProps is Record<PropertyKey, never>,
  // it means that TProps === JSX.IntrinsicAttributes.
  // @ts-expect-error
  return <Component {...props.additionalProps} />;
}
