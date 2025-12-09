export default 'query Query__errorsClientPointerField($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on Economist {\
      __typename,\
      id,\
    },\
  },\
}';