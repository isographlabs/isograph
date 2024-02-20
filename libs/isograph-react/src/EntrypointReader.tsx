import * as React from 'react';
import {
  ExtractReadFromStore,
  ExtractResolverProps,
  ExtractResolverResult,
  IsographEntrypoint,
  type FragmentReference,
  useResult,
} from './index';

export function EntrypointReader<
  TEntrypoint extends IsographEntrypoint<any, any, any>,
>(props: {
  queryReference: FragmentReference<
    ExtractReadFromStore<TEntrypoint>,
    ExtractResolverProps<TEntrypoint>,
    ExtractResolverResult<TEntrypoint>
  >;
}): ReturnType<React.FC<any>> {
  const Component = useResult(props.queryReference);
  return <Component />;
}
