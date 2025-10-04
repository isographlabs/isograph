export default 'query PetCheckinsCard($id: ID!, $skip: Int, $limit: Int) {\
  node____id___v_id: node(id: $id) {\
    ... on Pet {\
      __typename,\
      id,\
      checkins____skip___v_skip____limit___v_limit: checkins(skip: $skip, limit: $limit) {\
        id,\
        location,\
        time,\
      },\
    },\
  },\
}';