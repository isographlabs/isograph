import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Avatar as resolver } from '../../../avatar.tsx';

// the type, when read out (either via useLazyReference or via graph)
export type User__Avatar__outputType = (React.FC<any>);

const readerAst: ReaderAst<User__Avatar__param> = [
  {
    kind: "Scalar",
    fieldName: "name",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "avatarUrl",
    alias: null,
    arguments: null,
  },
];

export type User__Avatar__param = { data:
{
  name: (string | null),
  avatarUrl: string,
},
[index: string]: any };

const artifact: ReaderArtifact<
  User__Avatar__param,
  User__Avatar__param,
  User__Avatar__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "User.Avatar" },
};

export default artifact;
