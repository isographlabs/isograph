import * as React from 'react';
import { ExtractReadFromStore, IsographEntrypoint } from '../core/entrypoint';
import { FragmentReference } from '../core/FragmentReference';
import { NetworkRequestReaderOptions } from '../core/read';
import { useResult } from './useResult';

type IsExactlyIntrinsicAttributes<T> = T extends JSX.IntrinsicAttributes
  ? JSX.IntrinsicAttributes extends T
    ? true
    : false
  : false;

export function FragmentReader<
  TProps extends Record<any, any>,
  TEntrypoint extends IsographEntrypoint<any, React.FC<TProps>>,
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
