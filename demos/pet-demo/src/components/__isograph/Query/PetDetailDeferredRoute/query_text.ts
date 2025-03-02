export default 'query PetDetailDeferredRoute ($id: ID!) {\
  pet____id___v_id: pet(id: $id) {\
    id,\
    name,\
  },\
  topLevelField____input___o_name__s_ThisIsJustHereToTestObjectLiterals_c: topLevelField(input: { name: "ThisIsJustHereToTestObjectLiterals" }) {\
    __typename,\
  },\
}';