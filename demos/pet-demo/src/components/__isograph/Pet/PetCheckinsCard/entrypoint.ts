import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Pet__PetCheckinsCard__param} from './param_type';
import {Pet__PetCheckinsCard__output_type} from './output_type';
import readerResolver from './resolver_reader';
import refetchQuery0 from './__refetch__0';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["checkin_id", ] },
];

const queryText = 'query PetCheckinsCard ($count: Int!) {\
  id,\
  checkins {\
    id,\
    location,\
    time,\
  },\
}';

const normalizationAst: NormalizationAst = [
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
];
const artifact: IsographEntrypoint<
  Pet__PetCheckinsCard__param,
  Pet__PetCheckinsCard__output_type
> = {
  kind: "Entrypoint",
  queryText,
  normalizationAst,
  nestedRefetchQueries,
  readerArtifact: readerResolver,
};

export default artifact;
