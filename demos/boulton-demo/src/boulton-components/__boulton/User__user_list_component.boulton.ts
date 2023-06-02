import type {BoultonNonFetchableResolver, ReaderAst} from '@boulton/react';
import { user_list_component as resolver } from '../user_list.tsx';
import User__avatar_component from './User__avatar_component.boulton';

const readerAst: ReaderAst = [
  {
    kind: "Scalar",
    response_name: "id",
    alias: null,
  },
  {
    kind: "Scalar",
    response_name: "name",
    alias: null,
  },
  {
    kind: "Resolver",
    alias: "avatar_component",
    resolver: User__avatar_component,
    variant: "Component",
  },
];

const artifact: BoultonNonFetchableResolver = {
  kind: 'NonFetchableResolver',
  resolver,
  readerAst,
};

export default artifact;
