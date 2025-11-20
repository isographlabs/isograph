export type Query__HomePage__rawResponse = {
  viewer: {
    id: string,
    avatarUrl: string,
    login: string,
    name: (string | null),
    repositories____first___l_10____after___l_null: {
      edges: {
        node: {
          id: string,
          description: (string | null),
          forkCount: number,
          name: string,
          nameWithOwner: string,
          owner: {
            __typename: string,
            id: string,
            login: string,
          },
          pullRequests: {
            totalCount: number,
          },
          stargazerCount: number,
          watchers: {
            totalCount: number,
          },
        },
      },
      pageInfo: {
        endCursor?: (string | null),
        hasNextPage: boolean,
      },
    },
  },
}

