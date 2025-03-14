export default 'query User__refetch ($id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on User {\
      __typename,\
      id,\
      avatarUrl,\
      login,\
      name,\
      repositories____first___l_10____after___l_null: repositories(first: 10, after: null) {\
        edges {\
          node {\
            id,\
            description,\
            forkCount,\
            name,\
            nameWithOwner,\
            owner {\
              __typename,\
              id,\
              login,\
            },\
            pullRequests {\
              totalCount,\
            },\
            stargazerCount,\
            watchers {\
              totalCount,\
            },\
          },\
        },\
        pageInfo {\
          endCursor,\
          hasNextPage,\
        },\
      },\
    },\
  },\
}';