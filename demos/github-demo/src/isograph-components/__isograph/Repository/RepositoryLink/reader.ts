import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Repository__RepositoryLink__param } from './param_type.ts';
import { Repository__RepositoryLink__outputType } from './output_type.ts';
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

const artifact: ReaderArtifact<
  Repository__RepositoryLink__param,
  Repository__RepositoryLink__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "RepositoryLink",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Repository.RepositoryLink" },
};

export default artifact;
