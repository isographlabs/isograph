export default 'query BlogItemMoreDetail ($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on BlogItem {\
      __typename,\
      id,\
      moreContent,\
    },\
  },\
}';