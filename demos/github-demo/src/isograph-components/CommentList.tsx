import React from "react";

import { iso } from "@isograph/react";

import { ResolverParameterType as CommentListProps } from "@iso/PullRequest/CommentList/reader.isograph";
import { ResolverParameterType as IssueCommentProps } from "@iso/IssueComment/formattedCommentCreationDate/reader.isograph";

import { Card, CardContent } from "@mui/material";

export const formattedCommentCreationDate = iso<IssueCommentProps, string>`
  field IssueComment.formattedCommentCreationDate {
    createdAt,
  }
`((props) => {
  const date = new Date(props.createdAt);
  return date.toLocaleDateString("en-us", {
    year: "numeric",
    month: "numeric",
    day: "numeric",
  });
});

export const CommentList = iso<
  CommentListProps,
  ReturnType<typeof CommentListComponent>
>`
  field PullRequest.CommentList @component {
    comments(last: $last,) {
      edges {
        node {
          id,
          bodyText,
          formattedCommentCreationDate,
          author {
            login,
          },
        },
      },
    },
  }
`(CommentListComponent);

function CommentListComponent(props: CommentListProps) {
  const comments = [...props.data.comments.edges].reverse();

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
              {comment.author?.login} commented on{" "}
              {comment.formattedCommentCreationDate}
            </small>
          </p>
        </CardContent>
      </Card>
    );
  });
}
