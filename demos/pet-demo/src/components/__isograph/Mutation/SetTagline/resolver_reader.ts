import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Mutation__SetTagline__param } from './param_type';
import { setTagline as resolver } from '../../../Pet/PetTaglineCard';

const readerAst: ReaderAst<Mutation__SetTagline__param> = [
  {
    kind: "Linked",
    isFallible: false,
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
        isFallible: false,
        fieldName: "pet",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Scalar",
            isFallible: false,
            fieldName: "tagline",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
        ],
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Mutation__SetTagline__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "SetTagline",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
