export default 'query Query__firstNode($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Node {\
      __typename,\
      id,\
      ... on Pet {\
        __typename,\
        id,\
        name,\
      },\
    },\
  },\
}';