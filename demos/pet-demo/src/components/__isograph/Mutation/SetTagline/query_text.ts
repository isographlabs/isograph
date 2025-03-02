export default 'mutation SetTagline ($input: SetPetTaglineParams!) {\
  set_pet_tagline____input___v_input: set_pet_tagline(input: $input) {\
    pet {\
      id,\
      tagline,\
    },\
  },\
}';