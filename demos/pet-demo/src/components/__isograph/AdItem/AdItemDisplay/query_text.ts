export default 'query AdItemDisplay ($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on AdItem {\
      __typename,\
      id,\
      advertiser,\
      message,\
    },\
  },\
}';