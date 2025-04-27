export default 'query Query__custom_pet_refetch($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Pet {\
      __typename,\
      id,\
      age,\
      best_friend_relationship {\
        best_friend {\
          id,\
          name,\
          picture,\
        },\
        picture_together,\
      },\
      checkins____skip___l_null____limit___l_null: checkins(skip: null, limit: null) {\
        id,\
        location,\
        time,\
      },\
      favorite_phrase,\
      name,\
      nickname,\
      potential_new_best_friends {\
        id,\
        name,\
      },\
      stats {\
        cuteness,\
        energy,\
        hunger,\
        intelligence,\
        sociability,\
        weight,\
      },\
      tagline,\
    },\
  },\
}';