export type Query__Newsfeed__rawResponse = {
  viewer: {
    id: string,
    newsfeed____skip___l_0____limit___l_6: {
      __typename: "AdItem",
      id: string,
    } | {
      __typename: "BlogItem",
      id: string,
      author: string,
      content: string,
      image: {
        id: string,
      },
      title: string,
    },
  },
}

