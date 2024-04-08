import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { User__Avatar__param } from './param_type.ts';
import { User__Avatar__outputType } from './output_type.ts';
import { Avatar as resolver } from '../../../avatar.tsx';

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

const artifact: ReaderArtifact<
  User__Avatar__param,
  User__Avatar__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "User.Avatar" },
};

export default artifact;
