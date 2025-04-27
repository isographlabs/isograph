export default 'query Query__refetch_pet_stats($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    stats {\
      cuteness,\
      energy,\
      hunger,\
      intelligence,\
      sociability,\
      weight,\
    },\
  },\
}';