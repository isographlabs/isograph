export default 'query subquery ($id: ID!) {\
  query {\
    node____id___v_id: node(id: $id) {\
      __typename,\
      id,\
    },\
  },\
}';