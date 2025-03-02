export default 'query UserPage ($userLogin: String!) {\
  user____login___v_userLogin: user(login: $userLogin) {\
    id,\
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
  viewer {\
    id,\
    avatarUrl,\
    name,\
  },\
}';