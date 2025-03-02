export default 'query PetDetailRoute ($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
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
}';