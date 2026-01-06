export type User__RepositoryConnection__raw_response_type = {
  node____id___v_id?: ({
    __typename: "User",
    id: string,
    repositories____first___v_first____after___v_after: {
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
}

