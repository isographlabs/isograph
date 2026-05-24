export type User__RepositoryConnection__raw_response_type = {
  readonly node____id___v_id?: ({
    readonly __typename: "User",
    readonly id: string,
    readonly repositories____first___v_first____after___v_after: {
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
}

