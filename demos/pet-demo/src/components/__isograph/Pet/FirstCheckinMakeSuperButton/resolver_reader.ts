import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__FirstCheckinMakeSuperButton__param } from './param_type';
import { FirstCheckinMakeSuperButton as resolver } from '../../../PetMakeFirstCheckinSuperButton';
import ICheckin__make_super__refetch_reader from '../../ICheckin/make_super/refetch_reader';

const readerAst: ReaderAst<Pet__FirstCheckinMakeSuperButton__param> = [
  {
    kind: "Linked",
    fieldName: "checkins",
    alias: null,
    arguments: [
      [
        "skip",
        { kind: "Literal", value: 0 },
      ],

      [
        "limit",
        { kind: "Literal", value: 1 },
      ],
    ],
    selections: [
      {
        kind: "ImperativelyLoadedField",
        alias: "make_super",
        refetchReaderArtifact: ICheckin__make_super__refetch_reader,
        refetchQuery: 0,
        name: "make_super",
      },
      {
        kind: "Scalar",
        fieldName: "location",
        alias: null,
        arguments: null,
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Pet__FirstCheckinMakeSuperButton__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Pet.FirstCheckinMakeSuperButton",
  resolver,
  readerAst,
};

export default artifact;
