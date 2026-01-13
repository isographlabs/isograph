export default 'query nodeField($id: ID!) {\
  id,\
  node____id___v_id: node(id: $id) {\
    __typename,\
    id,\
  },\
}';