import type {IsographFetchableResolver, ReaderAst, FragmentReference} from '@isograph/react';
import { getRefRendererForName } from '@isograph/react';
import { foo_query as resolver } from '../user_list_page.tsx';

const queryText = 'query foo_query  {\
  users {\
    id,\
    name,\
  },\
}';

// TODO support changing this,
export type ReadFromStoreType = ResolverParameterType;

const normalizationAst = {notNeededForDemo: true};
const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    response_name: "users",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        response_name: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        response_name: "name",
        alias: null,
        arguments: null,
      },
    ],
  },
];

export type ResolverParameterType = {
  users: ({
    id: string,
    name: string,
  })[],
};

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
  convert: ((resolver, data) => resolver(data)),
};

export default artifact;
