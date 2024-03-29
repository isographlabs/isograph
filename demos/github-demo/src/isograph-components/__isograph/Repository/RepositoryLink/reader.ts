import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { RepositoryLink as resolver } from '../../../RepositoryLink.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type Repository__RepositoryLink__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

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

export type Repository__RepositoryLink__param = {
  id: string,
  name: string,
  owner: {
    login: string,
  },
};

const artifact: ReaderArtifact<
  Repository__RepositoryLink__param,
  Repository__RepositoryLink__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Repository.RepositoryLink" },
};

export default artifact;
