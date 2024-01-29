import type {IsographEntrypoint, ReaderAst, FragmentReference, NormalizationAst} from '@isograph/react';
const queryText = 'mutation Pet__set_pet_best_friend ($id: ID!, $new_best_friend_id: ID!) {\
set_pet_best_friend____id___id____new_best_friend_id___new_best_friend_id: set_pet_best_friend(id: $id, new_best_friend_id: $new_best_friend_id) {\
pet { \
  id,\
  best_friend_relationship {\
    best_friend {\
      id,\
      name,\
      picture,\
    },\
    picture_together,\
  },\
  checkins {\
    id,\
    location,\
    time,\
  },\
  favorite_phrase,\
  name,\
  potential_new_best_friends {\
    id,\
    name,\
  },\
  tagline,\
}}}';

const normalizationAst: NormalizationAst = [{
  kind: "Linked",
  fieldName: "set_pet_best_friend",
  arguments: [
    [
      "id",
      { kind: "Variable", name: "id" },
    ],

    [
      "new_best_friend_id",
      { kind: "Variable", name: "new_best_friend_id" },
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
        kind: "Linked",
        fieldName: "best_friend_relationship",
        arguments: null,
        selections: [
          {
            kind: "Linked",
            fieldName: "best_friend",
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "name",
                arguments: null,
              },
              {
                kind: "Scalar",
                fieldName: "picture",
                arguments: null,
              },
            ],
          },
          {
            kind: "Scalar",
            fieldName: "picture_together",
            arguments: null,
          },
        ],
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
        fieldName: "favorite_phrase",
        arguments: null,
      },
      {
        kind: "Scalar",
        fieldName: "name",
        arguments: null,
      },
      {
        kind: "Linked",
        fieldName: "potential_new_best_friends",
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            arguments: null,
          },
          {
            kind: "Scalar",
            fieldName: "name",
            arguments: null,
          },
        ],
      },
      {
        kind: "Scalar",
        fieldName: "tagline",
        arguments: null,
      },
    ],
    },
  ],
}];
const artifact: any = {
  kind: "RefetchQuery",
  queryText,
  normalizationAst,
};

export default artifact;
