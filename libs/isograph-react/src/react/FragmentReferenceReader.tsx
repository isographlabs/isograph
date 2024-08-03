import * as React from 'react';
import { ExtractReadFromStore, IsographEntrypoint } from '../core/entrypoint';
import { FragmentReference } from '../core/FragmentReference';
import { useResult } from './useResult';

export function FragmentReferenceReader<
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
      }
    : {
        fragmentReference: FragmentReference<
          ExtractReadFromStore<TEntrypoint>,
          React.FC<TProps>
        >;
        additionalProps: TProps;
      },
): React.ReactNode {
  const Component = useResult(props.fragmentReference);
  return <Component {...props.additionalProps} />;
}
