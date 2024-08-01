import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Checkin__CheckinDisplay__param } from './param_type';
import { CheckinDisplay as resolver } from '../../../PetCheckinsCard';
import ICheckin__make_super__refetch_reader from '../../ICheckin/make_super/refetch_reader';

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
    kind: "ImperativelyLoadedField",
    alias: "make_super",
    refetchReaderArtifact: ICheckin__make_super__refetch_reader,
    refetchQuery: 0,
    name: "make_super",
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
