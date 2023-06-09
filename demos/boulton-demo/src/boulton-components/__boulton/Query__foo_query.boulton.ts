import type {BoultonFetchableResolver, ReaderAst, FragmentReference} from '@boulton/react';
import { foo_query as resolver } from '../user_list.tsx';

const queryText = 'query foo_query  {\
  users {\
    id,\
    name,\
  },\
}';

const normalizationAst = {notNeededForDemo: true};
const readerAst: ReaderAst<ResolverParameterType> = [
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
  users: {
    id: string,
    name: string,
  },
};

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = FragmentReference<ResolverParameterType, ResolverReturnType, TReadOutType>;

const artifact: BoultonFetchableResolver<ResolverParameterType, ResolverReturnType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
convert: (x) => { return x; },
};

export default artifact;
