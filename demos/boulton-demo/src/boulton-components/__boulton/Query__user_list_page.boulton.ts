import type {BoultonFetchableResolver, ReaderAst, FragmentReference} from '@boulton/react';
import { user_list_page as resolver } from '../user_list_page.tsx';
import User__user_list_component, { ReadOutType as User__user_list_component__outputType } from './User__user_list_component.boulton';

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
        alias: "user_list_component",
        arguments: null,
        resolver: User__user_list_component,
        variant: "Component",
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  byah: string,
  users: ({
    id: string,
    user_list_component: User__user_list_component__outputType,
  })[],
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = React.FC<{ } & Object>;

const artifact: BoultonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
  convert: (x) => { return x; },
};

export default artifact;
