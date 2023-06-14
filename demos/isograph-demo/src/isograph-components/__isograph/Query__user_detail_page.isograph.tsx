import type {IsographFetchableResolver, ReaderAst, FragmentReference} from '@isograph/react';
import { getRefRendererForName } from '@isograph/react';
import { user_detail_page as resolver } from '../user_detail_page.tsx';
import User__user_profile_header, { ReadOutType as User__user_profile_header__outputType } from './User__user_profile_header.isograph';
import User__user_profile_with_details, { ReadOutType as User__user_profile_with_details__outputType } from './User__user_profile_with_details.isograph';

const queryText = 'query user_detail_page ($id: ID!) {\
  user__id_id: user(id: $id) {\
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

// TODO support changing this,
export type ReadFromStoreType = ResolverParameterType;

const normalizationAst = {notNeededForDemo: true};
const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Linked",
    response_name: "user",
    alias: null,
    arguments: {
      "id": "id",
    },
    selections: [
      {
        kind: "Scalar",
        response_name: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Resolver",
        alias: "user_profile_header",
        arguments: null,
        resolver: User__user_profile_header,
        variant: "Component",
      },
      {
        kind: "Resolver",
        alias: "user_profile_with_details",
        arguments: null,
        resolver: User__user_profile_with_details,
        variant: "Component",
      },
    ],
  },
];

export type ResolverParameterType = { data:
{
  user: {
    id: string,
    user_profile_header: User__user_profile_header__outputType,
    user_profile_with_details: User__user_profile_with_details__outputType,
  },
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (additionalRuntimeProps: Object | void) => (React.ReactElement<any, any> | null);

const artifact: IsographFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'FetchableResolver',
  queryText,
  normalizationAst,
  readerAst,
  resolver: resolver as any,
  convert: (() => {
    const RefRendererForName = getRefRendererForName('user_detail_page');
    return ((resolver, data) => additionalRuntimeProps => 
      {
        return <RefRendererForName 
          resolver={resolver}
          data={data}
          additionalRuntimeProps={additionalRuntimeProps}
        />;
      })
    })(),
};

export default artifact;
