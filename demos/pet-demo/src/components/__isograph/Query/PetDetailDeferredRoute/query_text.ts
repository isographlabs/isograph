export default 'query PetDetailDeferredRoute($id: ID!) {\
  namable {\
    __typename,\
  },\
  notImplemented {\
    __typename,\
  },\
  pet____id___v_id: pet(id: $id) {\
    id,\
    firstName,\
    lastName,\
  },\
  topLevelField____input___o_name__s_ThisIsJustHereToTestObjectLiterals_c: topLevelField(input: { name: "ThisIsJustHereToTestObjectLiterals" }) {\
    __typename,\
  },\
}';