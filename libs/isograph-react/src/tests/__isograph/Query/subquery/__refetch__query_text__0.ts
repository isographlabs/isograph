export default 'query Query____refetch($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Query {\
      __typename,\
      id,\
    },\
  },\
}';