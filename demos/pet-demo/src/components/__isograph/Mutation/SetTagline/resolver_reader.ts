import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Mutation__SetTagline__param } from './param_type';
import { setTagline as resolver } from '../../../Pet/PetTaglineCard';

const readerAst: ReaderAst<Mutation__SetTagline__param> = [
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
    selections: [
      {
        kind: "Linked",
        fieldName: "pet",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        selections: [
          {
            kind: "Scalar",
            fieldName: "tagline",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
        refetchQueryIndex: null,
      },
    ],
    refetchQueryIndex: null,
  },
];

const artifact: ComponentReaderArtifact<
  Mutation__SetTagline__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Mutation.SetTagline",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
