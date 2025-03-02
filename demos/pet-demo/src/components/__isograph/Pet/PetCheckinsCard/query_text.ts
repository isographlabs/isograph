export default 'query PetCheckinsCard ($skip: Int, $limit: Int, $id: ID!) {\
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