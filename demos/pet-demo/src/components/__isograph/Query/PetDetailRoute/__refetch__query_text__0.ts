export default 'query Query____refetch($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Pet {\
      __typename,\
      id,\
      __typename,\
      tagline,\
    },\
  },\
}';