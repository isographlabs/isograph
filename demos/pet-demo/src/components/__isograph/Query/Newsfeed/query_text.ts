export default 'query Newsfeed {\
  viewer {\
    id,\
    newsfeed____skip___l_0____limit___l_6: newsfeed(skip: 0, limit: 6) {\
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
}';