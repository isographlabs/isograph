export default 'query errors($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    __typename,\
    id,\
    ... on Economist {\
      __typename,\
      id,\
      name,\
    },\
  },\
}';