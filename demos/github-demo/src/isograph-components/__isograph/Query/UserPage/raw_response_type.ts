export type Query__UserPage__raw_response_type = {
  user____login___v_userLogin?: ({
    id: string,
    name?: (string | null),
    repositories____first___l_10____after___l_null: {
      edges?: (ReadonlyArray<({
        node?: ({
          id: string,
          description?: (string | null),
          forkCount: number,
          name: string,
          nameWithOwner: string,
          owner: {
            __typename: "Organization" | "User",
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
        } | null),
      } | null)> | null),
      pageInfo: {
        endCursor?: (string | null),
        hasNextPage: boolean,
      },
    },
  } | null),
  viewer: {
    id: string,
    avatarUrl: string,
    name?: (string | null),
  },
}

