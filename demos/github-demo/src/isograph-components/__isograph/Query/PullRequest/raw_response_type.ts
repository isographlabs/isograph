export type Query__PullRequest__rawResponse = {
  repository____owner___v_repositoryOwner____name___v_repositoryName: {
    id: string,
    pullRequest____number___v_pullRequestNumber: {
      id: string,
      bodyHTML: string,
      comments____last___l_10: {
        edges: {
          node: {
            id: string,
            author: {
              __typename: string,
              login: string,
            },
            bodyText: string,
            createdAt: string,
          },
        },
      },
      title: string,
    },
  },
  viewer: {
    id: string,
    avatarUrl: string,
    name?: (string | null),
  },
}

