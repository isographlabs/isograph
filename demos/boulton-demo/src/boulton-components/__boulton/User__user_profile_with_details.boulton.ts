import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { user_profile_with_details as resolver } from '../user_detail_page.tsx';
import BillingDetails__billing_details_component from './BillingDetails__billing_details_component.boulton';
import User__avatar_component from './User__avatar_component.boulton';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = React.FC<{ data: ResolverParameterType } & Object>;

const readerAst: ReaderAst<ReadFromStoreType> = [
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
  {
    kind: "Scalar",
    response_name: "email",
    alias: null,
    arguments: null,
  },
  {
    kind: "Resolver",
    alias: "avatar_component",
    arguments: null,
    resolver: User__avatar_component,
    variant: "Component",
  },
  {
    kind: "Linked",
    response_name: "billing_details",
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
        alias: "billing_details_component",
        arguments: null,
        resolver: BillingDetails__billing_details_component,
        variant: "Component",
      },
    ],
  },
];

const artifact: BoultonNonFetchableResolver = {
  kind: 'NonFetchableResolver',
  resolver,
  readerAst,
  convert: (x) => x,
            };

export default artifact;
