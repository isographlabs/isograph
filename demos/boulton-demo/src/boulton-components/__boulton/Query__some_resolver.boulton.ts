import type {BoultonFetchableResolver, ReaderAst} from '@boulton/react';
import { some_resolver as resolver } from '../user_list.tsx';

const queryText = 'query some_resolver  {\
  byah__foo_bar2: byah(foo: $bar2),\
}';

const normalizationAst = {notNeededForDemo: true};
const readerAst: ReaderAst = [
  {
    kind: "Scalar",
    response_name: "byah",
    alias: null,
    arguments: {
      "foo": "bar2",
    },
  },
];

// The type, when passed to the resolver (currently this is the raw response type, it should be the response type)
export type ResolverParameterType = {
  byah: string,
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
