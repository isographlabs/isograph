export default 'query PetByName ($name: String!) {\
  petByName____name___v_name: petByName(name: $name) {\
    id,\
    name,\
  },\
}';