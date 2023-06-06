import type {BoultonFetchableResolver, ReaderAst} from '@boulton/react';
import { user_list_page as resolver } from '../user_list.tsx';
import Query__some_resolver from './Query__some_resolver.boulton';
import User__user_list_component from './User__user_list_component.boulton';

const queryText = 'query user_list_page ($bar: String!, $bar2: String!) {\
  byah__foo_bar2: byah(foo: $bar2),\
  byah__foo_bar: byah(foo: $bar),\
  users {\
    avatar_url,\
    email,\
    id,\
    name,\
  },\
}';

const normalizationAst = {notNeededForDemo: true};
const readerAst: ReaderAst = [
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
  {
    kind: "Resolver",
    alias: "some_resolver",
    arguments: null,
    resolver: Query__some_resolver,
    variant: "Eager",
  },
];

export type ResolverParameterType = {
  byah: string,
  byah: string,
  users: {
    avatar_url: string,
    email: string,
    id: string,
    name: string,
  },
};

// The type, when returned from the resolver
type ResolverResponse = {
  foo: string
};

// The type, when read out
type UserResponse = {
  foo: string
};

const artifact: BoultonFetchableResolver<ResolverParameterType, ResolverResponse, UserResponse> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver,
};

export default artifact;
