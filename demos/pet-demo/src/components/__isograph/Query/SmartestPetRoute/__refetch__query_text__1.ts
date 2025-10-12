export default 'query Query__firstCheckin($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on ICheckin {\
      __typename,\
      id,\
      location,\
    },\
  },\
}';