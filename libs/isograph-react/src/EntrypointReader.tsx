import * as React from 'react';
import { ExtractReadFromStore, IsographEntrypoint } from './entrypoint';
import { FragmentReference } from './FragmentReference';
import { useResult } from './useResult';

export function EntrypointReader<
  TProps extends Record<any, any>,
  TEntrypoint extends IsographEntrypoint<any, React.FC<TProps>>,
>(
  props: TProps extends Record<string, never>
    ? {
        queryReference: FragmentReference<
          ExtractReadFromStore<TEntrypoint>,
          React.FC<TProps>
        >;
        additionalProps?: TProps;
      }
    : {
        queryReference: FragmentReference<
          ExtractReadFromStore<TEntrypoint>,
          React.FC<TProps>
        >;
        additionalProps: TProps;
      },
): React.ReactNode {
  const Component = useResult(props.queryReference);
  return <Component {...props.additionalProps} />;
}
