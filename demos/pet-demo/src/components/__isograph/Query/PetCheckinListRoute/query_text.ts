export default 'query PetCheckinListRoute ($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    id,\
    checkins____skip___l_0____limit___l_1: checkins(skip: 0, limit: 1) {\
      id,\
      location,\
    },\
    name,\
  },\
}';