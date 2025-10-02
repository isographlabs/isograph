import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Mutation__MututalBestFriendSetterMutation__param } from './param_type';
import { setMututalBestFriend as resolver } from '../../../Pet/MutualBestFriendSetter';
import Mutation__MutualBestFriendSetterOtherSide__entrypoint from '../../Mutation/MutualBestFriendSetterOtherSide/entrypoint';

const readerAst: ReaderAst<Mutation__MututalBestFriendSetterMutation__param> = [
  {
    kind: "Linked",
    fieldName: "set_pet_best_friend",
    alias: null,
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
                ],
              },
            ],
          },
        ],
      },
    ],
  },
  {
    kind: "LoadablySelectedField",
    alias: "MutualBestFriendSetterOtherSide",
    name: "MutualBestFriendSetterOtherSide",
    queryArguments: null,
    refetchReaderAst: [
    ],
    entrypoint: Mutation__MutualBestFriendSetterOtherSide__entrypoint,
  },
];

const artifact: ComponentReaderArtifact<
  Mutation__MututalBestFriendSetterMutation__param,
  ExtractSecondParam<typeof resolver>
> = {
  kind: "ComponentReaderArtifact",
  fieldName: "Mutation.MututalBestFriendSetterMutation",
  resolver,
  readerAst,
  hasUpdatable: false,
};

export default artifact;
