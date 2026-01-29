import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { User__Avatar__param } from './param_type';
import { Avatar as resolver } from '../../../avatar';

const readerAst: ReaderAst<User__Avatar__param> = [
  {
    kind: "Scalar",
    isFallible: true,
    fieldName: "name",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "avatarUrl",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
];

const artifact = (): ComponentReaderArtifact<
  User__Avatar__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "Avatar",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
