import * as React from 'react';
import {
  ExtractReadFromStore,
  IsographEntrypoint,
  type FragmentReference,
  useResult,
} from './index';

export function EntrypointReader<
  TEntrypoint extends IsographEntrypoint<any, React.FC<any>>,
>(props: {
  queryReference: FragmentReference<
    ExtractReadFromStore<TEntrypoint>,
    React.FC<any>
  >;
  additionalProps?: any | void;
}): ReturnType<React.FC<any>> {
  const Component = useResult(props.queryReference);
  return <Component {...props.additionalProps} />;
}
