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
    alias: "link",
  },
];

const artifact: EagerReaderArtifact<
  { data: any, parameters: Record<PropertyKey, never> },
  Link | null
> = {
  kind: "EagerReaderArtifact",
  fieldName: "NewsfeedItem.asBlogItem",
  resolver: ({ data }) => data.__typename === "BlogItem" ? data.link : null,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
