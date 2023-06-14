import type { IsographnNonFetchableResolver, ReaderAst } from "@isograph/react";
import { avatar_component as resolver } from "../avatar.tsx";

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (
  additionalRuntimeProps: Object | void
) => React.ReactElement<any, any> | null;

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
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
    kind: "Scalar",
    response_name: "avatar_url",
    alias: null,
    arguments: null,
  },
];

export type ResolverParameterType = {
  data: {
    name: string;
    email: string;
    avatar_url: string;
  };
  [index: string]: any;
};

// The type, wheIsographned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: BoultonNonFetchableResolver<
  ReadFromStoreType,
  ResolverParameterType,
  ReadOutType
> = {
  kind: "NonFetchableResolver",
  resolver: resolver as any,
  readerAst,
};

export default artifact;
