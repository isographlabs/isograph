export type Query__Newsfeed__raw_response_type = {
  readonly viewer: {
    readonly id: string,
    readonly newsfeed____skip___l_0____limit___l_6: ReadonlyArray<{
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
  },
}

