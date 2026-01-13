export default 'query startUpdate($id: ID!) {\
  id,\
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