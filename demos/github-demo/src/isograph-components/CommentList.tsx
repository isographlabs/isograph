import React from 'react';

import { iso } from '@iso';

import { Card, CardContent } from '@mui/material';

export const formattedCommentCreationDate = iso(`
  field IssueComment.formattedCommentCreationDate {
    createdAt
  }
`)(({ data }) => {
  const date = new Date(data.createdAt);
  return date.toLocaleDateString('en-us', {
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
  });
});

export const CommentList = iso(`
  field PullRequest.CommentList @component {
    comments(last: $last) {
      edges {
        node {
          id
          bodyText
          formattedCommentCreationDate
          author {
            login
          }
        }
      }
    }
  }
`)(function CommentListComponent({ data }) {
  const comments = [...(data.comments.edges ?? [])].reverse();

  return comments.map((commentNode) => {
    const comment = commentNode?.node;
    if (comment == null) {
      return;
    }
    return (
      <Card key={comment.id} variant="outlined">
        <CardContent>
          {comment.bodyText}
          <p>
            <small>
              {comment.author?.login} commented on{' '}
              {comment.formattedCommentCreationDate}
            </small>
          </p>
        </CardContent>
      </Card>
    );
  });
});
