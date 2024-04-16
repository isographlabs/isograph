import type {ReaderArtifact, ReaderAst} from '@isograph/react';
import { Checkin__CheckinDisplay__param } from './param_type.ts';
import { Checkin__CheckinDisplay__outputType } from './output_type.ts';
import { CheckinDisplay as resolver } from '../../../PetCheckinsCard.tsx';
import Checkin__make_super from '../make_super/reader';

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

const artifact: ReaderArtifact<
  Checkin__CheckinDisplay__param,
  Checkin__CheckinDisplay__outputType
> = {
  kind: "ReaderArtifact",
  fieldName: "CheckinDisplay",
  resolver: resolver as any,
  readerAst,
  variant: { kind: "Component", componentName: "Checkin.CheckinDisplay" },
};

export default artifact;
