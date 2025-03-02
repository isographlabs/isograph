export default 'query RepositoryPage ($repositoryName: String!, $repositoryOwner: String!, $first: Int!) {\
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
              id,\
              __typename,\
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