import * as React from 'react';
import { ExtractReadFromStore, IsographEntrypoint } from '../core/entrypoint';
import { FragmentReference } from '../core/FragmentReference';
import { useResult } from './useResult';
import { NetworkRequestReaderOptions } from '../core/read';

export function FragmentReader<
  TProps extends Record<any, any>,
  TEntrypoint extends IsographEntrypoint<any, React.FC<TProps>>,
>(
  props: TProps extends Record<string, never>
    ? {
        fragmentReference: FragmentReference<
          ExtractReadFromStore<TEntrypoint>,
          React.FC<{}>
        >;
        additionalProps?: TProps;
        networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
      }
    : {
        fragmentReference: FragmentReference<
          ExtractReadFromStore<TEntrypoint>,
          React.FC<TProps>
        >;
        additionalProps: TProps;
        networkRequestOptions?: Partial<NetworkRequestReaderOptions>;
      },
): React.ReactNode {
  const Component = useResult(
    props.fragmentReference,
    props.networkRequestOptions,
  );
  return <Component {...props.additionalProps} />;
}
