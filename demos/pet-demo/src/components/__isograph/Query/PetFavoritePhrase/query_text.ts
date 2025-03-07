export default 'query PetFavoritePhrase ($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    id,\
    favorite_phrase,\
    name,\
  },\
}';