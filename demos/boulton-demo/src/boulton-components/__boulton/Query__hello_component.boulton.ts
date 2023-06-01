import type {BoultonFetchableResolver, ReaderAst} from '@boulton/react';
import { hello_component as resolver } from '../hello.tsx';
import { User__avatar_component } from './User__avatar_component';

const queryText = 'query hello_component {\
  hello,\
  current_user {\
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
    response_name: "hello",
    alias: null,
  },
  {
    kind: "Linked",
    response_name: "current_user",
    alias: null,
    selections: [
      {
        kind: "Scalar",
        response_name: "id",
        alias: null,
      },
      {
        kind: "Resolver",
        alias: "avatar_component",
        resolver: User__avatar_component,
      },
    ],
  },
];

// The type, when passed to the resolver (currently this is the raw response type, it should be the response type)
type FragmentType = {
  hello: string,
  current_user: {
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
