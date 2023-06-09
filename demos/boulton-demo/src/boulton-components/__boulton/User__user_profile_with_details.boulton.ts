import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { user_profile_with_details as resolver } from '../user_detail_page.tsx';
import BillingDetails__billing_details_component, { ReadOutType as BillingDetails__billing_details_component__outputType } from './BillingDetails__billing_details_component.boulton';
import User__avatar_component, { ReadOutType as User__avatar_component__outputType } from './User__avatar_component.boulton';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = React.FC<{ } & Object>;

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

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

export type ResolverParameterType = { data: {
  id: string,
  name: string,
  email: string,
  avatar_component: User__avatar_component__outputType,
  billing_details: {
    id: string,
    billing_details_component: BillingDetails__billing_details_component__outputType,
  },
} };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: BoultonNonFetchableResolver = {
  kind: 'NonFetchableResolver',
  resolver,
  readerAst,
  convert: (x) => x,
            };

export default artifact;
