export default 'mutation Query__set_pet_tagline($input: SetPetTaglineParams!) {\
  set_pet_tagline____input___v_input: set_pet_tagline(input: $input) {\
    pet {\
      id,\
      age,\
      best_friend_relationship {\
        best_friend {\
          id,\
          firstName,\
          lastName,\
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
      firstName,\
      lastName,\
      nickname,\
      potential_new_best_friends {\
        id,\
        firstName,\
        lastName,\
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