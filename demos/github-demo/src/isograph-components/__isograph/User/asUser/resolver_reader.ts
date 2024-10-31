import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { User__asUser__param } from './param_type';
import { User__asUser__output_type } from './output_type';

const readerAst: ReaderAst<User__asUser__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "__typename",
    alias: null,
    arguments: null,
  },
];

const artifact: EagerReaderArtifact<
  User__asUser__param,
  User__asUser__output_type
> = {
  kind: "EagerReaderArtifact",
  resolver: ({ data }) => data.__typename === "User" ? { __link: data.id } : null,
  readerAst,
};

export default artifact;
