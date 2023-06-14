import type { IsographNonFetchableResolver, ReaderAst } from "@isograph/react";
import { last_four_digits as resolver } from "../last_four_digits.ts";

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    response_name: "credit_card_number",
    alias: null,
    arguments: null,
  },
];

export type ResolverParameterType = {
  credit_card_number: string;
};

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: IsographNonFetchableResolver<
  ReadFromStoreType,
  ResolverParameterType,
  ReadOutType
> = {
  kind: "NonFetchableResolver",
  resolver: resolver as any,
  readerAst,
};

export default artifact;
