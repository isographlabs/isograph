import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Pet__PetCheckinsCard__param} from './param_type';
import {Pet__PetCheckinsCard__output_type} from './output_type';
import readerResolver from './resolver_reader';
import refetchQuery0 from './__refetch__0';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [
  { artifact: refetchQuery0, allowedVariables: ["checkin_id", ] },
];

const queryText = 'query PetCheckinsCard ($skip: Int, $limit: Int, $id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Pet {\
      __typename,\
      id,\
      checkins____skip___v_skip____limit___v_limit: checkins(skip: $skip, limit: $limit) {\
        id,\
        location,\
        time,\
      },\
    },\
  },\
}';

const normalizationAst: NormalizationAst = [
  {
    kind: "Linked",
    fieldName: "node",
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    selections: [
      {
        kind: "InlineFragment",
        type: "Pet",
        selections: [
          {
            kind: "Scalar",
            fieldName: "__typename",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Linked",
            fieldName: "checkins",
            arguments: [
              [
                "skip",
                { kind: "Variable", name: "skip" },
              ],

              [
                "limit",
                { kind: "Variable", name: "limit" },
              ],
            ],
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
        ],
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
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
