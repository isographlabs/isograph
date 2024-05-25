import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst, RefetchQueryNormalizationArtifact} from '@isograph/react';
import { Checkin__CheckinDisplay__param } from './param_type';
import { CheckinDisplay as resolver } from '../../../PetCheckinsCard.tsx';
import ICheckin__make_super from '../../ICheckin/make_super/refetch_reader';

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
    // @ts-ignore
    readerArtifact: ICheckin__make_super,
    refetchQuery: 0,
  },
];

const artifact: ComponentReaderArtifact<
  Checkin__CheckinDisplay__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Checkin.CheckinDisplay",
  resolver,
  readerAst,
};

export default artifact;
