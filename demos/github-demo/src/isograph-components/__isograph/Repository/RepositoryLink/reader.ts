import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { RepositoryLink as resolver } from '../../../RepositoryLink.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = Repository__RepositoryLink__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
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

export type Repository__RepositoryLink__param = { data:
{
  id: string,
  name: string,
  owner: {
    login: string,
  },
},
[index: string]: any };

// The type, when returned from the resolver
export type ResolverReturnType = ReturnType<typeof resolver>;

const artifact: ReaderArtifact<ReadFromStoreType, Repository__RepositoryLink__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Repository.RepositoryLink" },
};

export default artifact;
