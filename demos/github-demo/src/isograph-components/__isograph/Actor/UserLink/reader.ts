import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Actor__UserLink__param } from './param_type.ts';
import { Actor__UserLink__outputType } from './output_type.ts';
import { UserLink as resolver } from '../../../UserLink.tsx';

const readerAst: ReaderAst<Actor__UserLink__param> = [
  {
    kind: "Scalar",
    fieldName: "login",
    alias: null,
    arguments: null,
  },
];

const artifact: ReaderArtifact<
  Actor__UserLink__param,
  Actor__UserLink__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "UserLink",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Actor.UserLink" },
};

export default artifact;
