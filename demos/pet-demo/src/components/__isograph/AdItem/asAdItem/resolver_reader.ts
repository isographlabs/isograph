import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';

const readerAst: ReaderAst<{ data: any, parameters: Record<PropertyKey, never> }> = [
  {
    kind: "Scalar",
    fieldName: "__typename",
    alias: null,
    arguments: null,
  },
  {
    kind: "Link",
    alias: "__link",
  },
];

const artifact: EagerReaderArtifact<
  { data: any, parameters: Record<PropertyKey, never> },
  boolean
> = {
  kind: "EagerReaderArtifact",
  resolver: ({ data }) => data.__typename === "AdItem" ? data.__link : null,
  readerAst,
};

export default artifact;
