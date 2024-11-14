import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Mutation__SetTagline__param } from './param_type';
import { setTagline as resolver } from '../../../PetTaglineCard';

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
    selections: [
      {
        kind: "Linked",
        fieldName: "pet",
        alias: null,
        arguments: null,
        condition: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "tagline",
            alias: null,
            arguments: null,
          },
        ],
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Mutation__SetTagline__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  componentName: "Mutation.SetTagline",
  resolver,
  readerAst,
};

export default artifact;
