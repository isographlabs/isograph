export default 'query nicknameErrors($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    __typename,\
    id,\
    ... on Economist {\
      __typename,\
      id,\
      nickname,\
    },\
  },\
}';