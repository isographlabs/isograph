import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Actor__UserLink__param } from './param_type';
import { UserLink as resolver } from '../../../UserLink';

const readerAst: ReaderAst<Actor__UserLink__param> = [
  {
    kind: "Scalar",
    fieldName: "login",
    alias: null,
    arguments: null,
  },
];

const artifact: ComponentReaderArtifact<
  Actor__UserLink__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Actor.UserLink",
  resolver,
  readerAst,
};

export default artifact;
