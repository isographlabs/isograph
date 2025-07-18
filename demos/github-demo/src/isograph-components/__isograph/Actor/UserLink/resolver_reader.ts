import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Actor__UserLink__param } from './param_type';
import { UserLink as resolver } from '../../../UserLink';
import Actor__asUser__resolver_reader from '../../Actor/asUser/resolver_reader';

const readerAst: ReaderAst<Actor__UserLink__param> = [
  {
    kind: "Scalar",
    fieldName: "login",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Linked",
    fieldName: "asUser",
    alias: null,
    arguments: null,
    condition: Actor__asUser__resolver_reader,
    isUpdatable: false,
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        fieldName: "twitterUsername",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: ComponentReaderArtifact<
  Actor__UserLink__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Actor.UserLink",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
