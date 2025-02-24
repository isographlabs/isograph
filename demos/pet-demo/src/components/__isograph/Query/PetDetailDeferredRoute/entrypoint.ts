import type {IsographEntrypoint, NormalizationAst, RefetchQueryNormalizationArtifactWrapper} from '@isograph/react';
import {Query__PetDetailDeferredRoute__param} from './param_type';
import {Query__PetDetailDeferredRoute__output_type} from './output_type';
import readerResolver from './resolver_reader';
const nestedRefetchQueries: RefetchQueryNormalizationArtifactWrapper[] = [];

const queryText = 'query PetDetailDeferredRoute ($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    id,\
    name,\
  },\
  topLevelField____input___o_name__s_ThisIsJustHereToTestObjectLiterals_c: topLevelField(input: { name: "ThisIsJustHereToTestObjectLiterals" }) {\
    __typename,\
  },\
}';

const normalizationAst: NormalizationAst = {
  kind: "NormalizationAst",
  selections: [
    {
      kind: "Linked",
      fieldName: "pet",
      arguments: [
        [
          "id",
          { kind: "Variable", name: "id" },
        ],
      ],
      concreteType: "Pet",
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
      kind: "Linked",
      fieldName: "topLevelField",
      arguments: [
        [
          "input",
          {
            kind: "Object",
            value: [
              [
                "name",
                { kind: "String", value: "ThisIsJustHereToTestObjectLiterals" },
              ],

            ]
          },
        ],
      ],
      concreteType: "TopLevelField",
      selections: [
        {
          kind: "Scalar",
          fieldName: "__typename",
          arguments: null,
        },
      ],
    },
  ],
};
const artifact: IsographEntrypoint<
  Query__PetDetailDeferredRoute__param,
  Query__PetDetailDeferredRoute__output_type,
  NormalizationAst
> = {
  kind: "Entrypoint",
  networkRequestInfo: {
    kind: "NetworkRequestInfo",
    queryText,
    normalizationAst,
  },
  concreteType: "Query",
  readerWithRefetchQueries: {
    kind: "ReaderWithRefetchQueries",
    nestedRefetchQueries,
    readerArtifact: readerResolver,
  },
};

export default artifact;
