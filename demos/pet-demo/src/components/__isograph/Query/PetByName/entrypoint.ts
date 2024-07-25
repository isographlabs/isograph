import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PetByName__param} from './param_type';
import {Query__PetByName__output_type} from './output_type';
import readerResolver from './resolver_reader';
import refetchQuery0 from './__refetch__0';
import refetchQuery1 from './__refetch__1';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["id", ] },
  { artifact: refetchQuery1, allowedVariables: ["checkin_id", ] },
];

const queryText = 'query PetByName ($name: String!) {\
  petByName____name___v_name: petByName(name: $name) {\
    id,\
    checkins {\
      id,\
      location,\
      time,\
    },\
    name,\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "petByName",
    arguments: [
      [
        "name",
        { kind: "Variable", name: "name" },
      ],
    ],
    selections: [
      {
        kind: "Scalar",
        fieldName: "id",
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "checkins",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "location",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "time",
            arguments: null,
          },
        ],
      },
      {
        kind: "Scalar",
        fieldName: "name",
        arguments: null,
      },
    ],
  },
];
const artifact: IsographEntrypoint<
  Query__PetByName__param,
  Query__PetByName__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
