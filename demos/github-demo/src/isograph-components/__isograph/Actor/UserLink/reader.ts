import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { UserLink as resolver } from '../../../UserLink.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type Actor__UserLink__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

const readerAst: ReaderAst<Actor__UserLink__param> = [
  {
    kind: "Scalar",
    fieldName: "login",
    alias: null,
    arguments: null,
  },
];

export type Actor__UserLink__param = {
  login: string,
};

const artifact: ReaderArtifact<
  Actor__UserLink__param,
  Actor__UserLink__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Actor.UserLink" },
};

export default artifact;
