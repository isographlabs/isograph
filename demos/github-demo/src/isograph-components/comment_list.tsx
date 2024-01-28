import React from "react";

import { iso } from "@isograph/react";

import { ResolverParameterType as CommentListProps } from "@iso/PullRequest/CommentList/reader.isograph";
import { ResolverParameterType as IssueCommentProps } from "@iso/IssueComment/formatted_comment_creation_date/reader.isograph";

import { Card, CardContent } from "@mui/material";

export const formatted_comment_creation_date = iso<IssueCommentProps, string>`
  IssueComment.formatted_comment_creation_date @eager {
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
  PullRequest.CommentList @component {
    comments(last: $last,) {
      edges {
        node {
          id,
          bodyText,
          formatted_comment_creation_date,
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
              {comment.formatted_comment_creation_date}
            </small>
          </p>
        </CardContent>
      </Card>
    );
  });
}
