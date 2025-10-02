import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Mutation__MutualBestFriendSetterOtherSide__param } from './param_type';
import { SomeThing as resolver } from '../../../Pet/MutualBestFriendSetter';

const readerAst: ReaderAst<Mutation__MutualBestFriendSetterOtherSide__param> = [
  {
    kind: "Linked",
    fieldName: "set_pet_best_friend",
    alias: null,
    arguments: [
      [
        "id",
        { kind: "Variable", name: "pet_id" },
      ],

      [
        "new_best_friend_id",
        { kind: "Variable", name: "new_best_friend_id" },
      ],
    ],
    condition: null,
    isUpdatable: false,
    refetchQueryIndex: null,
    selections: [
      {
        kind: "Linked",
        fieldName: "pet",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "id",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
          {
            kind: "Scalar",
            fieldName: "name",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
          {
            kind: "Linked",
            fieldName: "best_friend_relationship",
            alias: null,
            arguments: null,
            condition: null,
            isUpdatable: false,
            refetchQueryIndex: null,
            selections: [
              {
                kind: "Linked",
                fieldName: "best_friend",
                alias: null,
                arguments: null,
                condition: null,
                isUpdatable: false,
                refetchQueryIndex: null,
                selections: [
                  {
                    kind: "Scalar",
                    fieldName: "id",
                    alias: null,
                    arguments: null,
                    isUpdatable: false,
                  },
                  {
                    kind: "Scalar",
                    fieldName: "name",
                    alias: null,
                    arguments: null,
                    isUpdatable: false,
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
];

const artifact: ComponentReaderArtifact<
  Mutation__MutualBestFriendSetterOtherSide__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Mutation.MutualBestFriendSetterOtherSide",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
