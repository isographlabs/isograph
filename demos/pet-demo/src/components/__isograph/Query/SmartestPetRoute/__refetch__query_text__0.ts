export default 'query Query__smartestPet($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Pet {\
      __typename,\
      id,\
      checkins____limit___l_1: checkins(limit: 1) {\
        id,\
      },\
      firstName,\
      lastName,\
      picture,\
      stats {\
        intelligence,\
      },\
    },\
  },\
}';