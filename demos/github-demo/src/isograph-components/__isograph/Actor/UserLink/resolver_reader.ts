import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Actor__UserLink__param } from './param_type';
import { UserLink as resolver } from '../../../UserLink';
import Actor__asUser__resolver_reader from '../../Actor/asUser/resolver_reader';

const readerAst: ReaderAst<Actor__UserLink__param> = [
  {
    kind: "Scalar",
    isFallible: false,
    fieldName: "login",
    alias: null,
    arguments: null,
    isUpdatable: false,
  },
  {
    kind: "Linked",
    isFallible: false,
    fieldName: "asUser",
    alias: null,
    arguments: null,
    condition: Actor__asUser__resolver_reader,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        isFallible: false,
        fieldName: "id",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
      {
        kind: "Scalar",
        isFallible: true,
        fieldName: "twitterUsername",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Actor__UserLink__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "UserLink",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
