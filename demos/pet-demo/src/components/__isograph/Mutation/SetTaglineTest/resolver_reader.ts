import type { EagerReaderArtifact, ReaderAst } from '@isograph/react';
import { Mutation__SetTaglineTest__param } from './param_type';
import { Mutation__SetTaglineTest__output_type } from './output_type';
import { setTagline as resolver } from '../../../Pet/PetTaglineCard2';
import Mutation__TestLazyReference__entrypoint from '../../Mutation/TestLazyReference/entrypoint';

const readerAst: ReaderAst<Mutation__SetTaglineTest__param> = [
  {
    kind: "Linked",
    fieldName: "set_pet_tagline",
    alias: null,
    arguments: [
      [
        "input",
        { kind: "Variable", name: "input" },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Linked",
        fieldName: "pet",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
      },
    ],
  },
  {
    kind: "LoadablySelectedField",
    alias: "TestLazyReference",
    name: "TestLazyReference",
    queryArguments: null,
    refetchReaderAst: [
    ],
    entrypoint: Mutation__TestLazyReference__entrypoint,
  },
];

const artifact: EagerReaderArtifact<
  Mutation__SetTaglineTest__param,
  Mutation__SetTaglineTest__output_type
> = {
  kind: "EagerReaderArtifact",
  fieldName: "Mutation.SetTaglineTest",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
