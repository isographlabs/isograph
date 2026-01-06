export type Query__PullRequest__raw_response_type = {
  repository____owner___v_repositoryOwner____name___v_repositoryName?: ({
    id: string,
    pullRequest____number___v_pullRequestNumber?: ({
      id: string,
      bodyHTML: string,
      comments____last___l_10: {
        edges?: (ReadonlyArray<({
          node?: ({
            id: string,
            author?: ({
              __typename: "Bot" | "EnterpriseUserAccount" | "Mannequin" | "Organization" | "User",
              login: string,
            } | null),
            bodyText: string,
            createdAt: string,
          } | null),
        } | null)> | null),
      },
      title: string,
    } | null),
  } | null),
  viewer: {
    id: string,
    avatarUrl: string,
    name?: (string | null),
  },
}

