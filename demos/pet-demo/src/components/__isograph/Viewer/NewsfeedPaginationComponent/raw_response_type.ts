export type Viewer__NewsfeedPaginationComponent__rawResponse = {
  node____id___v_id: {
    __typename: 'Viewer',
    id: string,
    newsfeed____skip___v_skip____limit___v_limit: {
      __typename: 'AdItem',
      id: string,
    } | {
      __typename: 'BlogItem',
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

