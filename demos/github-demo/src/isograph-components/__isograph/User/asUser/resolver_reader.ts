import type { EagerReaderArtifact, ReaderAst, Link } from '@isograph/react';

const readerAst: ReaderAst<{ data: any, parameters: Record<PropertyKey, never> }> = [
  {
    kind: "Scalar",
    fieldName: "__typename",
    alias: null,
    arguments: null,
  },
  {
    kind: "Link",
    alias: "link",
  },
];

const artifact: EagerReaderArtifact<
  { data: any, parameters: Record<PropertyKey, never> },
  Link | null
> = {
  kind: "EagerReaderArtifact",
  fieldName: "User.asUser",
  resolver: ({ data }) => data.__typename === "User" ? data.link : null,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
