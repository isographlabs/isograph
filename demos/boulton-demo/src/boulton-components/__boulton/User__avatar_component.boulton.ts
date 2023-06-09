import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { avatar_component as resolver } from '../avatar.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = React.FC<{ data: ResolverParameterType } & Object>;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    response_name: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    response_name: "email",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    response_name: "avatar_url",
    alias: null,
    arguments: null,
  },
];

const artifact: BoultonNonFetchableResolver = {
  kind: 'NonFetchableResolver',
  resolver,
  readerAst,
  convert: (x) => x,
            };

export default artifact;
