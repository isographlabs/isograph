export default 'mutation Query__set_best_friend($id: ID!, $new_best_friend_id: ID!) {\
  set_pet_best_friend____id___v_id____new_best_friend_id___v_new_best_friend_id: set_pet_best_friend(id: $id, new_best_friend_id: $new_best_friend_id) {\
    pet {\
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