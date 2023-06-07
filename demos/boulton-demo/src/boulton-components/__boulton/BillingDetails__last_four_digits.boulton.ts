import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { last_four_digits as resolver } from '../last_four_digits.ts';

// TODO generate actual types
export type ReadOutType = string;

const readerAst: ReaderAst = [
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
};

export default artifact;
