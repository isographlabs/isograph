import type { EagerReaderArtifact, ReaderAst, Link } from '@isograph/react';

const readerAst: ReaderAst<{ data: any, parameters: Record<PropertyKey, never> }> = [
  {
    kind: "Scalar",
    isFallible: false,
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

const artifact = (): EagerReaderArtifact<
  { data: any, parameters: Record<PropertyKey, never> },
  Link<"BlogItem"> | null
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "asBlogItem",
  resolver: ({ firstParameter }) => firstParameter.data.__typename === "BlogItem" ? firstParameter.data.__link : null,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
