import type {BoultonFetchableResolver, ReaderAst} from '@boulton/react';
import { user_detail_page as resolver } from '../user_detail_page.tsx';
import User__user_profile_with_details from './User__user_profile_with_details.boulton';

const queryText = 'query user_detail_page  {\
  current_user {\
    avatar_url,\
    email,\
    id,\
    name,\
    billing_details {\
      address,\
      card_brand,\
      credit_card_number,\
      expiration_date,\
      id,\
    },\
  },\
}';

const normalizationAst = {notNeededForDemo: true};
const readerAst: ReaderAst = [
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
        alias: "user_profile_with_details",
        resolver: User__user_profile_with_details,
        variant: "Component",
      },
    ],
  },
];

// The type, when passed to the resolver (currently this is the raw response type, it should be the response type)
export type ResolverParameterType = {
  current_user: {
    avatar_url: string,
    email: string,
    id: string,
    name: string,
    billing_details: {
      address: string,
      card_brand: string,
      credit_card_number: string,
      expiration_date: string,
      id: string,
    },
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

const artifact: BoultonFetchableResolver<ResolverParamaterType, ResolverResponse, UserResponse> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver,
};

export default artifact;
