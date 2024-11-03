import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Actor__UserLink__param } from './param_type';
import { UserLink as resolver } from '../../../UserLink';
import User__asUser__resolver_reader from '../../User/asUser/resolver_reader';

const readerAst: ReaderAst<Actor__UserLink__param> = [
  {
    kind: "Linked",
    fieldName: "asUser",
    alias: null,
    arguments: null,
    condition: User__asUser__resolver_reader,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "login",
        alias: null,
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "twitterUsername",
        alias: null,
        arguments: null,
      },
    ],
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
