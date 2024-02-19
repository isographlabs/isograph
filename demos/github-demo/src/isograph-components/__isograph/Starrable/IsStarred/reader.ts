import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { IsStarred as resolver } from '../../../RepositoryDetail.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type Starrable__IsStarred__outputType = (React.FC<any>);

const readerAst: ReaderAst<Starrable__IsStarred__param> = [
  {
    kind: "Scalar",
    fieldName: "stargazerCount",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "viewerHasStarred",
    alias: null,
    arguments: null,
  },
];

export type Starrable__IsStarred__param = { data:
{
  stargazerCount: number,
  viewerHasStarred: boolean,
},
[index: string]: any };

const artifact: ReaderArtifact<
  Starrable__IsStarred__param,
  Starrable__IsStarred__param,
  Starrable__IsStarred__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Starrable.IsStarred" },
};

export default artifact;
