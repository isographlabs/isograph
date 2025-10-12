import type { EagerReaderArtifact, ReaderAst, Link } from '@isograph/react';

const readerAst: ReaderAst<{ data: any, parameters: Record<PropertyKey, never> }> = [
  {
    kind: "Scalar",
    fieldName: "__typename",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Link",
    alias: "__link",
  },
];

const artifact: EagerReaderArtifact<
  { data: any, parameters: Record<PropertyKey, never> },
  Link<"Economist"> | null
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Node.asEconomist",
  resolver: ({ data }) => data.__typename === "Economist" ? data.__link : null,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
