export type Query__UserPage__raw_response_type = {
  readonly user____login___v_userLogin?: ({
    readonly id: string,
    readonly name?: (string | null),
    readonly repositories____first___l_10____after___l_null: {
      readonly edges?: (ReadonlyArray<({
        readonly node?: ({
          readonly id: string,
          readonly description?: (string | null),
          readonly forkCount: number,
          readonly name: string,
          readonly nameWithOwner: string,
          readonly owner: {
            readonly __typename: "Organization" | "User",
            readonly id: string,
            readonly login: string,
          },
          readonly pullRequests: {
            readonly totalCount: number,
          },
          readonly stargazerCount: number,
          readonly watchers: {
            readonly totalCount: number,
          },
        } | null),
      } | null)> | null),
      readonly pageInfo: {
        readonly endCursor?: (string | null),
        readonly hasNextPage: boolean,
      },
    },
  } | null),
  readonly viewer: {
    readonly id: string,
    readonly avatarUrl: unknown,
    readonly name?: (string | null),
  },
}

