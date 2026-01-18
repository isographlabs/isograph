import type {ComponentReaderArtifact, ExtractSecondParam, ReaderAst } from '@isograph/react';
import { Mutation__MutualBestFriendSetterOtherSide__param } from './param_type';
import { SomeThing as resolver } from '../../../Pet/MutualBestFriendSetter';
import Pet__fullName__resolver_reader from '../../Pet/fullName/resolver_reader';

const readerAst: ReaderAst<Mutation__MutualBestFriendSetterOtherSide__param> = [
  {
    kind: "Linked",
    isFallible: false,
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
            kind: "Resolver",
            alias: "fullName",
            arguments: null,
            readerArtifact: Pet__fullName__resolver_reader,
            usedRefetchQueries: [],
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
                ],
              },
            ],
          },
        ],
      },
    ],
  },
];

const artifact = (): ComponentReaderArtifact<
  Mutation__MutualBestFriendSetterOtherSide__param,
  ExtractSecondParam<typeof resolver>
> => ({
  kind: "ComponentReaderArtifact",
  fieldName: "MutualBestFriendSetterOtherSide",
  resolver,
  readerAst,
  hasUpdatable: false,
});

export default artifact;
