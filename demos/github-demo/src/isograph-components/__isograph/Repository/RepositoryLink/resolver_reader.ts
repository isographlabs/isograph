import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Repository__RepositoryLink__param } from './param_type';
import { RepositoryLink as resolver } from '../../../RepositoryLink.tsx';

const readerAst: ReaderAst<Repository__RepositoryLink__param> = [
  {
    kind: "Scalar",
    fieldName: "id",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "Linked",
    fieldName: "owner",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "login",
        alias: null,
        arguments: null,
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Repository__RepositoryLink__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Repository.RepositoryLink",
  resolver,
  readerAst,
};

export default artifact;
