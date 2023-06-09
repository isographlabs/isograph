import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { billing_details_component as resolver } from '../billing_details_component.tsx';
import BillingDetails__last_four_digits from './BillingDetails__last_four_digits.boulton';

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

const artifact: BoultonNonFetchableResolver = {
  kind: 'NonFetchableResolver',
  resolver,
  readerAst,
  convert: (x) => x,
            };

export default artifact;
