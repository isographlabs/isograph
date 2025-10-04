export default 'query NewsfeedPaginationComponent($skip: Int!, $limit: Int!, $id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Viewer {\
      __typename,\
      id,\
      newsfeed____skip___v_skip____limit___v_limit: newsfeed(skip: $skip, limit: $limit) {\
        __typename,\
        ... on AdItem {\
          __typename,\
          id,\
        },\
        ... on BlogItem {\
          __typename,\
          id,\
          author,\
          content,\
          image {\
            id,\
          },\
          title,\
        },\
      },\
    },\
  },\
}';