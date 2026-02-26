export type Query__PullRequest__raw_response_type = {
  readonly repository____owner___v_repositoryOwner____name___v_repositoryName?: ({
    readonly id: string,
    readonly pullRequest____number___v_pullRequestNumber?: ({
      readonly id: string,
      readonly bodyHTML: unknown,
      readonly comments____last___l_10: {
        readonly edges?: (ReadonlyArray<({
          readonly node?: ({
            readonly id: string,
            readonly author?: ({
              readonly __typename: "Bot" | "EnterpriseUserAccount" | "Mannequin" | "Organization" | "User",
              readonly login: string,
            } | null),
            readonly bodyText: string,
            readonly createdAt: unknown,
          } | null),
        } | null)> | null),
      },
      readonly title: string,
    } | null),
  } | null),
  readonly viewer: {
    readonly id: string,
    readonly avatarUrl: unknown,
    readonly name?: (string | null),
  },
}

