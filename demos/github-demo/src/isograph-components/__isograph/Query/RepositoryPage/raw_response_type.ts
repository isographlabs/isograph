export type Query__RepositoryPage__raw_response_type = {
  readonly repository____name___v_repositoryName____owner___v_repositoryOwner?: ({
    readonly id: string,
    readonly nameWithOwner: string,
    readonly parent?: ({
      readonly id: string,
      readonly name: string,
      readonly nameWithOwner: string,
      readonly owner: {
        readonly __typename: "Organization" | "User",
        readonly id: string,
        readonly login: string,
      },
    } | null),
    readonly pullRequests____last___v_first: {
      readonly edges?: (ReadonlyArray<({
        readonly node?: ({
          readonly id: string,
          readonly author?: ({
            readonly __typename: "User",
            readonly id: string,
            readonly login: string,
            readonly twitterUsername?: (string | null),
          } | null),
          readonly closed: boolean,
          readonly createdAt: unknown,
          readonly number: number,
          readonly repository: {
            readonly id: string,
            readonly name: string,
            readonly owner: {
              readonly __typename: "Organization" | "User",
              readonly id: string,
              readonly login: string,
            },
          },
          readonly title: string,
          readonly totalCommentsCount?: (number | null),
        } | null),
      } | null)> | null),
    },
    readonly stargazerCount: number,
    readonly viewerHasStarred: boolean,
  } | null),
  readonly viewer: {
    readonly id: string,
    readonly avatarUrl: unknown,
    readonly name?: (string | null),
  },
}

