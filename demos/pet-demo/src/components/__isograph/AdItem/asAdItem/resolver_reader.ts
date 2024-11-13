import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';

const readerAst: ReaderAst<{ data: any, parameters: Record<PropertyKey, never> }> = [
  {
    kind: "Scalar",
    fieldName: "__typename",
    alias: null,
    arguments: null,
  },
];

const artifact: EagerReaderArtifact<
  { data: any, parameters: Record<PropertyKey, never> },
  boolean
> = {
  kind: "EagerReaderArtifact",
  resolver: ({ data }) => data.__typename === "AdItem",
  readerAst,
};

export default artifact;
