import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Pet__FirstCheckinMakeSuperButton__param } from './param_type';
import { FirstCheckinMakeSuperButton as resolver } from '../../../Pet/PetMakeFirstCheckinSuperButton';
import Checkin__make_super__refetch_reader from '../../Checkin/make_super/refetch_reader';

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
    condition: null,
    isUpdatable: false,
    selections: [
      {
        kind: "ImperativelyLoadedField",
        alias: "make_super",
        refetchReaderArtifact: Checkin__make_super__refetch_reader,
        refetchQueryIndex: 0,
        name: "make_super",
      },
      {
        kind: "Scalar",
        fieldName: "location",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: ComponentReaderArtifact<
  Pet__FirstCheckinMakeSuperButton__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Pet.FirstCheckinMakeSuperButton",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
