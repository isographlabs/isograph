import * as React from 'react';
import { ExtractReadFromStore, IsographEntrypoint } from './entrypoint';
import { FragmentReference } from './FragmentReference';
import { useResult } from './useResult';

export function EntrypointReader<
  TProps extends Record<any, any>,
  TEntrypoint extends IsographEntrypoint<any, React.FC<TProps>>,
>(props: {
  queryReference: FragmentReference<
    ExtractReadFromStore<TEntrypoint>,
    React.FC<TProps>
  >;
  // TODO make additionalProps optional if TProps extends Record<string, never>
  // perhaps with a type overload
  additionalProps: TProps;
}): React.ReactNode {
  const Component = useResult(props.queryReference);
  return <Component {...props.additionalProps} />;
}
