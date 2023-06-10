import type {BoultonFetchableResolver, ReaderAst, FragmentReference} from '@boulton/react';
import { getRefRendererForName } from '@boulton/react';
import { user_list_page as resolver } from '../user_list_page.tsx';
import User__user_detail, { ReadOutType as User__user_detail__outputType } from './User__user_detail.boulton';

const queryText = 'query user_list_page ($bar: String!) {\
  byah__foo_bar: byah(foo: $bar),\
  users {\
    avatar_url,\
    email,\
    id,\
    name,\
  },\
}';

// TODO support changing this,
export type ReadFromStoreType = ResolverParameterType;

const normalizationAst = {notNeededForDemo: true};
const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    response_name: "byah",
    alias: null,
    arguments: {
      "foo": "bar",
    },
  },
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
        kind: "Resolver",
        alias: "user_detail",
        arguments: null,
        resolver: User__user_detail,
        variant: "Component",
      },
    ],
  },
];

export type ResolverParameterType = {
  byah: string,
  users: ({
    id: string,
    user_detail: User__user_detail__outputType,
  })[],
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

const artifact: BoultonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
  convert: ((resolver, data) => resolver(data)),
};

export default artifact;
