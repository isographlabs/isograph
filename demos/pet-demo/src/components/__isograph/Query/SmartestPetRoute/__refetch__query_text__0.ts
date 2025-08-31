export default 'query Query__smartestPet($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Pet {\
      __typename,\
      id,\
      name,\
      picture,\
      stats {\
        intelligence,\
      },\
    },\
  },\
}';