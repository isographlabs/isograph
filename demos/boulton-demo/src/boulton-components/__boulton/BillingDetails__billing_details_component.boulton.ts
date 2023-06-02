import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { billing_details_component as resolver } from '../billing_details_component.tsx';

const readerAst: ReaderAst = [
  {
    kind: "Scalar",
    response_name: "id",
    alias: null,
  },
  {
    kind: "Scalar",
    response_name: "card_brand",
    alias: null,
  },
  {
    kind: "Scalar",
    response_name: "credit_card_number",
    alias: null,
  },
  {
    kind: "Scalar",
    response_name: "expiration_date",
    alias: null,
  },
  {
    kind: "Scalar",
    response_name: "address",
    alias: null,
  },
];

const artifact: BoultonNonFetchableResolver = {
  kind: 'NonFetchableResolver',
  resolver,
  readerAst,
};

export default artifact;
