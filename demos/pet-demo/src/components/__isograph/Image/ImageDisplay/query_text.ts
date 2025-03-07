export default 'query ImageDisplay ($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Image {\
      __typename,\
      id,\
      url,\
    },\
  },\
}';