import React from "react";

import { iso } from "@isograph/react";

import { ResolverParameterType as CommentListProps } from "./__isograph/PullRequest__comment_list.isograph";
import { ResolverParameterType as IssueCommentProps } from "./__isograph/IssueComment__formatted_comment_creation_date.isograph";

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

export const comment_list = iso<
  CommentListProps,
  ReturnType<typeof CommentList>
>`
  PullRequest.comment_list @component {
    id,
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
`(CommentList);

function CommentList(props: CommentListProps) {
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
              {comment.author.login} commented on{" "}
              {comment.formatted_comment_creation_date}
            </small>
          </p>
        </CardContent>
      </Card>
    );
  });
}
