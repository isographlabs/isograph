import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { billing_details_component as resolver } from '../billing_details_component.tsx';
import BillingDetails__last_four_digits, { ReadOutType as BillingDetails__last_four_digits__outputType } from './BillingDetails__last_four_digits.boulton';

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
    response_name: "card_brand",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    response_name: "credit_card_number",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    response_name: "expiration_date",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    response_name: "address",
    alias: null,
    arguments: null,
  },
  {
    kind: "Resolver",
    alias: "last_four_digits",
    arguments: null,
    resolver: BillingDetails__last_four_digits,
    variant: "Eager",
  },
];

export type ResolverParameterType = { data:
{
  id: string,
  card_brand: string,
  credit_card_number: string,
  expiration_date: string,
  address: string,
  last_four_digits: BillingDetails__last_four_digits__outputType,
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: BoultonNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
// is this needed?
              convert: (x) => {throw new Error('convert non fetchable')},
};

export default artifact;
