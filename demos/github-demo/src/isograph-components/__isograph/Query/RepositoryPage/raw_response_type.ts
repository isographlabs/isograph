export type Query__RepositoryPage__raw_response_type = {
  repository____name___v_repositoryName____owner___v_repositoryOwner?: ({
    id: string,
    nameWithOwner: string,
    parent?: ({
      id: string,
      name: string,
      nameWithOwner: string,
      owner: {
        __typename: "Organization" | "User",
        id: string,
        login: string,
      },
    } | null),
    pullRequests____last___v_first: {
      edges?: (ReadonlyArray<({
        node?: ({
          id: string,
          author?: ({
            __typename: "User",
            id: string,
            login: string,
            twitterUsername?: (string | null),
          } | null),
          closed: boolean,
          createdAt: string,
          number: number,
          repository: {
            id: string,
            name: string,
            owner: {
              __typename: "Organization" | "User",
              id: string,
              login: string,
            },
          },
          title: string,
          totalCommentsCount?: (number | null),
        } | null),
      } | null)> | null),
    },
    stargazerCount: number,
    viewerHasStarred: boolean,
  } | null),
  viewer: {
    id: string,
    avatarUrl: string,
    name?: (string | null),
  },
}

