import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { User__Avatar__param } from './param_type';
import { Avatar as resolver } from '../../../avatar';

const readerAst: ReaderAst<User__Avatar__param> = [
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "avatarUrl",
    alias: null,
    arguments: null,
  },
];

const artifact: ComponentReaderArtifact<
  User__Avatar__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "User.Avatar",
  resolver,
  readerAst,
};

export default artifact;
