import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Mutation__SetTagline__param} from './param_type';
import {Mutation__SetTagline__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'mutation SetTagline ($input: SetPetTaglineParams!) {\
  set_pet_tagline____input___v_input: set_pet_tagline(input: $input) {\
    pet {\
      id,\
      tagline,\
    },\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "set_pet_tagline",
    arguments: [
      [
        "input",
        { kind: "Variable", name: "input" },
      ],
    ],
    selections: [
      {
        kind: "Linked",
        fieldName: "pet",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "tagline",
            arguments: null,
          },
        ],
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  Mutation__SetTagline__param,
  Mutation__SetTagline__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
