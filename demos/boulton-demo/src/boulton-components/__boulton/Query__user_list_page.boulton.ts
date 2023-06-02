import type {BoultonFetchableResolver, ReaderAst} from '@boulton/react';
import { user_list_page as resolver } from '../user_list.tsx';
import User__user_list_component from './User__user_list_component.boulton';

const queryText = 'query user_list_page {\
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
    kind: "Linked",
    response_name: "users",
    alias: null,
    selections: [
      {
        kind: "Scalar",
        response_name: "id",
        alias: null,
      },
      {
        kind: "Resolver",
        alias: "user_list_component",
        resolver: User__user_list_component,
      },
    ],
  },
];

// The type, when passed to the resolver (currently this is the raw response type, it should be the response type)
type FragmentType = {
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

const artifact: BoultonFetchableResolver<FragmentType, ResolverResponse, UserResponse> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver,
};

export default artifact;
