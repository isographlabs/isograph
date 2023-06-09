import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { last_four_digits as resolver } from '../last_four_digits.ts';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    response_name: "credit_card_number",
    alias: null,
    arguments: null,
  },
];

const artifact: BoultonNonFetchableResolver = {
  kind: 'NonFetchableResolver',
  resolver,
  readerAst,
  convert: (x) => x,
            };

export default artifact;
