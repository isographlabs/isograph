export default 'query meNameSuccessor {\
  id,\
  me {\
    id,\
    name,\
    successor {\
      id,\
      successor {\
        id,\
        name,\
      },\
    },\
  },\
}';