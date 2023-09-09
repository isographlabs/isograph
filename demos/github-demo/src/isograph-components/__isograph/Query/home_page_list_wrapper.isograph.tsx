import type {IsographNonFetchableResolver, ReaderAst} from '@isograph/react';
const resolver = x => x;
import Query__home_page_list, { ReadOutType as Query__home_page_list__outputType } from './home_page_list.isograph';
import UserStatus____update_user_bio, { ReadOutType as UserStatus____update_user_bio__outputType } from '../UserStatus/__update_user_bio.isograph';

// the type, when read out (either via useLazyReference or via graph)
export type ReadOutType = ResolverReturnType;

// TODO support changing this
export type ReadFromStoreType = ResolverParameterType;

const readerAst: ReaderAst<ReadFromStoreType> = [
  {
    kind: "Resolver",
    alias: "home_page_list",
    arguments: null,
    resolver: Query__home_page_list,
    variant: "Component",
    usedRefetchQueries: [0, ],
  },
  {
    kind: "Linked",
    fieldName: "viewer",
    alias: null,
    arguments: null,
    selections: [
      {
        kind: "Linked",
        fieldName: "status",
        alias: null,
        arguments: null,
        selections: [
          {
            kind: "Scalar",
            fieldName: "emoji",
            alias: null,
            arguments: null,
          },
          {
            kind: "MutationField",
            alias: "__update_user_bio",
            resolver: UserStatus____update_user_bio,
            refetchQuery: 1,
          },
          {
            kind: "Linked",
            fieldName: "user",
            alias: null,
            arguments: null,
            selections: [
              {
                kind: "Scalar",
                fieldName: "id",
                alias: null,
                arguments: null,
              },
              {
                kind: "Linked",
                fieldName: "repositories",
                alias: null,
                arguments: [
                  {
                    argumentName: "last",
                    variableName: "first",
                  },
                ],
                selections: [
                  {
                    kind: "Scalar",
                    fieldName: "__typename",
                    alias: null,
                    arguments: null,
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

export type ResolverParameterType = {
  home_page_list: Query__home_page_list__outputType,
  viewer: {
    status: ({
      emoji: (string | null),
      __update_user_bio: UserStatus____update_user_bio__outputType,
      user: {
        id: string,
        repositories: {
          __typename: string,
        },
      },
    } | null),
  },
};

// The type, when returned from the resolver
export type ResolverReturnType = ResolverParameterType;

const artifact: IsographNonFetchableResolver<ReadFromStoreType, ResolverParameterType, ReadOutType> = {
  kind: 'NonFetchableResolver',
  resolver: resolver as any,
  readerAst,
};

export default artifact;
