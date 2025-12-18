export default 'query RepositoryPage($first: Int!, $repositoryName: String!, $repositoryOwner: String!) {\
  repository____name___v_repositoryName____owner___v_repositoryOwner: repository(name: $repositoryName, owner: $repositoryOwner) {\
    id,\
    nameWithOwner,\
    parent {\
      id,\
      name,\
      nameWithOwner,\
      owner {\
        __typename,\
        id,\
        login,\
      },\
    },\
    pullRequests____last___v_first: pullRequests(last: $first) {\
      edges {\
        node {\
          id,\
          author {\
            __typename,\
            login,\
            ... on User {\
              __typename,\
              id,\
              twitterUsername,\
            },\
          },\
          closed,\
          createdAt,\
          number,\
          repository {\
            id,\
            name,\
            owner {\
              __typename,\
              id,\
              login,\
            },\
          },\
          title,\
          totalCommentsCount,\
        },\
      },\
    },\
    stargazerCount,\
    viewerHasStarred,\
  },\
  viewer {\
    id,\
    avatarUrl,\
    name,\
  },\
}';