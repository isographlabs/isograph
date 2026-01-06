import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Query__PetDetailDeferredRoute__param } from './param_type';
import { PetDetailDeferredRouteComponent as resolver } from '../../../PetDetailDeferredRoute';
import Pet__PetDetailDeferredRouteInnerComponent__resolver_reader from '../../Pet/PetDetailDeferredRouteInnerComponent/resolver_reader';

const readerAst: ReaderAst<Query__PetDetailDeferredRoute__param> = [
  {
    kind: "Linked",
    fieldName: "pet",
    alias: null,
    arguments: [
      [
        "id",
        { kind: "Variable", name: "id" },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Resolver",
        alias: "PetDetailDeferredRouteInnerComponent",
        arguments: null,
        readerArtifact: Pet__PetDetailDeferredRouteInnerComponent__resolver_reader,
        usedRefetchQueries: [],
      },
    ],
  },
  {
    kind: "Linked",
    fieldName: "topLevelField",
    alias: null,
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
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "__typename",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
  {
    kind: "Linked",
    fieldName: "namable",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "__typename",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
  {
    kind: "Linked",
    fieldName: "notImplemented",
    alias: null,
    arguments: null,
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Scalar",
        fieldName: "__typename",
        alias: null,
        arguments: null,
        isUpdatable: false,
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Query__PetDetailDeferredRoute__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "PetDetailDeferredRoute",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
