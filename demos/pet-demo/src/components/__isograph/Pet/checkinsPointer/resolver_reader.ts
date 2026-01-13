import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Pet__checkinsPointer__param } from './param_type';
import { Pet__checkinsPointer__output_type } from './output_type';
import { checkinsPointer as resolver } from '../../../SmartestPet';

const readerAst: ReaderAst<Pet__checkinsPointer__param> = [
  {
    kind: "Linked",
    fieldName: "checkins",
    alias: null,
    arguments: [
      [
        "limit",
        { kind: "Literal", value: 2 },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Link",
        alias: "__link",
      },
    ],
  },
];

const artifact = (): EagerReaderArtifact<
  Pet__checkinsPointer__param,
  Pet__checkinsPointer__output_type
> => ({
  kind: "EagerReaderArtifact",
  fieldName: "checkinsPointer",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
