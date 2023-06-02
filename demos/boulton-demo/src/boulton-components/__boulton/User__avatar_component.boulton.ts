import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { avatar_component as resolver } from '../avatar.tsx';

const readerAst: ReaderAst = [
  {
    kind: "Scalar",
    response_name: "name",
    alias: null,
  },
  {
    kind: "Scalar",
    response_name: "email",
    alias: null,
  },
  {
    kind: "Scalar",
    response_name: "avatar_url",
    alias: null,
  },
];

const artifact: BoultonNonFetchableResolver = {
  kind: 'NonFetchableResolver',
  resolver,
  readerAst,
};

export default artifact;
