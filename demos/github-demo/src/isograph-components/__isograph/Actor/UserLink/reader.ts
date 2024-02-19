import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { UserLink as resolver } from '../../../UserLink.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = (React.FC<any>);

export type ReadFromStoreType = Actor__UserLink__param;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Scalar",
    fieldName: "login",
    alias: null,
    arguments: null,
  },
];

export type Actor__UserLink__param = { data:
{
  login: string,
},
[index: string]: any };

const artifact: ReaderArtifact<ReadFromStoreType, Actor__UserLink__param, ReadOutType> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Actor.UserLink" },
};

export default artifact;
