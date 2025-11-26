export default 'query PetFavoritePhrase2($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    id,\
    favorite_phrase,\
    firstName,\
    lastName,\
  },\
}';