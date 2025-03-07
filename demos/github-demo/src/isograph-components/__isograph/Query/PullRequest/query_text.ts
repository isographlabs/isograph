export default 'query PullRequest ($repositoryOwner: String!, $repositoryName: String!, $pullRequestNumber: Int!) {\
  repository____owner___v_repositoryOwner____name___v_repositoryName: repository(owner: $repositoryOwner, name: $repositoryName) {\
    id,\
    pullRequest____number___v_pullRequestNumber: pullRequest(number: $pullRequestNumber) {\
      id,\
      bodyHTML,\
      comments____last___l_10: comments(last: 10) {\
        edges {\
          node {\
            id,\
            author {\
              __typename,\
              login,\
            },\
            bodyText,\
            createdAt,\
          },\
        },\
      },\
      title,\
    },\
  },\
  viewer {\
    id,\
    avatarUrl,\
    name,\
  },\
}';