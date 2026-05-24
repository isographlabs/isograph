export type Viewer__NewsfeedPaginationComponent__raw_response_type = {
  readonly node____id___v_id?: ({
    readonly __typename: "Viewer",
    readonly id: string,
    readonly newsfeed____skip___v_skip____limit___v_limit: ReadonlyArray<{
      readonly __typename: "AdItem",
      readonly id: string,
    } | {
      readonly __typename: "BlogItem",
      readonly id: string,
      readonly author: string,
      readonly content: string,
      readonly image?: ({
        readonly id: string,
      } | null),
      readonly title: string,
    }>,
  } | null),
}

