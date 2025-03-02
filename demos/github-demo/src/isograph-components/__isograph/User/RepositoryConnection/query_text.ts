export default 'query RepositoryConnection ($first: Int, $after: String, $id: ID!) {\
  node____id___v_id: node(id: $id) {\
    ... on User {\
      __typename,\
      id,\
      repositories____first___v_first____after___v_after: repositories(first: $first, after: $after) {\
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