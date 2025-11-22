export type User__RepositoryConnection__rawResponse = {
  node____id___v_id: {
    __typename: 'User',
    id: string,
    repositories____first___v_first____after___v_after: {
      edges: {
        node: {
          id: string,
          description?: (string | null),
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

