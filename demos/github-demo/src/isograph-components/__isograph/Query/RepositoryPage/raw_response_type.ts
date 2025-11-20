export type Query__RepositoryPage__rawResponse = {
  repository____name___v_repositoryName____owner___v_repositoryOwner: {
    id: string,
    nameWithOwner: string,
    parent: {
      id: string,
      name: string,
      nameWithOwner: string,
      owner: {
        __typename: string,
        id: string,
        login: string,
      },
    },
    pullRequests____last___v_first: {
      edges: {
        node: {
          id: string,
          author: {
            __typename: string,
            login: string,
            __typename: 'User',
            id: string,
            twitterUsername: (string | null),
          },
          closed: boolean,
          createdAt: string,
          number: number,
          repository: {
            id: string,
            name: string,
            owner: {
              __typename: string,
              id: string,
              login: string,
            },
          },
          title: string,
          totalCommentsCount: (number | null),
        },
      },
    },
    stargazerCount: number,
    viewerHasStarred: boolean,
  },
  viewer: {
    id: string,
    avatarUrl: string,
    name?: (string | null),
  },
}

