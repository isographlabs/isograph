import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Pet__firstCheckin__param } from './param_type';
import { Pet__firstCheckin__output_type } from './output_type';
import { firstCheckin as resolver } from '../../../SmartestPet';

const readerAst: ReaderAst<Pet__firstCheckin__param> = [
  {
    kind: "Linked",
    fieldName: "checkins",
    alias: null,
    arguments: [
      [
        "limit",
        { kind: "Literal", value: 1 },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Link",
        alias: "link",
      },
    ],
  },
];

const artifact: EagerReaderArtifact<
  Pet__firstCheckin__param,
  Pet__firstCheckin__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Pet.firstCheckin",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
