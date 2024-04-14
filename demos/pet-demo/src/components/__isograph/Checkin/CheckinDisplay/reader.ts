import type {ReaderArtifact, ReaderAst, ExtractSecondParam} from '@isograph/react';
import { CheckinDisplay as resolver } from '../../../PetCheckinsCard.tsx';
import Checkin__make_super, { Checkin__make_super__outputType} from '../make_super/reader';

// the type, when read out (either via useLazyReference or via graph)
export type Checkin__CheckinDisplay__outputType = (React.FC<ExtractSecondParam<typeof resolver>>);

const readerAst: ReaderAst<Checkin__CheckinDisplay__param> = [
  {
    kind: "Scalar",
    fieldName: "location",
    alias: null,
    arguments: null,
  },
  {
    kind: "Scalar",
    fieldName: "time",
    alias: null,
    arguments: null,
  },
  {
    kind: "MutationField",
    alias: "make_super",
    readerArtifact: Checkin__make_super,
    refetchQuery: 0,
  },
];

export type Checkin__CheckinDisplay__param = {
  location: string,
  time: string,
  make_super: Checkin__make_super__outputType,
};

const artifact: ReaderArtifact<
  Checkin__CheckinDisplay__param,
  Checkin__CheckinDisplay__outputType
> = {
  kind: "ReaderArtifact",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Checkin.CheckinDisplay" },
};

export default artifact;
