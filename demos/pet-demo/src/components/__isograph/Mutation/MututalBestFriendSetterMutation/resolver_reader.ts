import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Mutation__MututalBestFriendSetterMutation__param } from './param_type';
import { setMututalBestFriend as resolver } from '../../../Pet/MutualBestFriendSetter';
import Mutation__MutualBestFriendSetterOtherSide__entrypoint from '../../Mutation/MutualBestFriendSetterOtherSide/entrypoint';
import Pet__Avatar__resolver_reader from '../../Pet/Avatar/resolver_reader';
import Pet__fullName__resolver_reader from '../../Pet/fullName/resolver_reader';

const readerAst: ReaderAst<Mutation__MututalBestFriendSetterMutation__param> = [
  {
    kind: "Linked",
    isFallible: false,
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
        isFallible: false,
        fieldName: "pet",
        alias: null,
        arguments: null,
        condition: null,
        isUpdatable: false,
        refetchQueryIndex: null,
        selections: [
          {
            kind: "Scalar",
            isFallible: false,
            fieldName: "id",
            alias: null,
            arguments: null,
            isUpdatable: false,
          },
          {
            kind: "Linked",
            isFallible: true,
            fieldName: "best_friend_relationship",
            alias: null,
            arguments: null,
            condition: null,
            isUpdatable: false,
            refetchQueryIndex: null,
            selections: [
              {
                kind: "Scalar",
                isFallible: true,
                fieldName: "picture_together",
                alias: null,
                arguments: null,
                isUpdatable: false,
              },
              {
                kind: "Linked",
                isFallible: false,
                fieldName: "best_friend",
                alias: null,
                arguments: null,
                condition: null,
                isUpdatable: false,
                refetchQueryIndex: null,
                selections: [
                  {
                    kind: "Scalar",
                    isFallible: false,
                    fieldName: "id",
                    alias: null,
                    arguments: null,
                    isUpdatable: false,
                  },
                  {
                    kind: "Resolver",
                    alias: "fullName",
                    arguments: null,
                    readerArtifact: Pet__fullName__resolver_reader,
                    usedRefetchQueries: [],
                  },
                  {
                    kind: "Resolver",
                    alias: "Avatar",
                    arguments: null,
                    readerArtifact: Pet__Avatar__resolver_reader,
                    usedRefetchQueries: [],
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

const artifact = (): ComponentReaderArtifact<
  Mutation__MututalBestFriendSetterMutation__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "MututalBestFriendSetterMutation",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
