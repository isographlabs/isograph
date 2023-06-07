import type {BoultonFetchableResolver, ReaderAst} from '@boulton/react';
import { user_list_page as resolver } from '../user_list.tsx';
import User__user_list_component, { ResolverOutputType as User__user_list_component__outputType } from './User__user_list_component.boulton';

const queryText = 'query user_list_page ($bar: String!, $bar2: String!) {\
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
];

export type ResolverParameterType = {
  byah: string,
  users: {
    id: string,
    user_list_component: User__user_list_component__outputType,
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
