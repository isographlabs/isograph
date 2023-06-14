import { bDeclare } from "@isograph/react";
import type { ResolverParameterType as PullRequestTableParams } from "./__isograph/PullRequestConnection__pull_request_table.isograph";
import type { ResolverParameterType as CreatedAtFormattedType } from "./__isograph/PullRequest__created_at_formatted.isograph";

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
} from "@mui/material";

export const created_at_formatted = bDeclare<CreatedAtFormattedType, string>`
  PullRequest.created_at_formatted @eager {
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

export const pull_request_table = bDeclare<
  PullRequestTableParams,
  ReturnType<typeof PullRequestTable>
>`
  PullRequestConnection.pull_request_table @component {
    edges {
      node {
        pull_request_link,
        number,
        id,
        title,
        author {
          user_link,
          login,
        },
        closed,
        totalCommentsCount,
        created_at_formatted,
      },
    },
  }
`(PullRequestTable);

function PullRequestTable(props: PullRequestTableParams) {
  const reversedPullRequests = [...props.data.edges].reverse();
  return (
    <>
      <h2>Pull Requests</h2>
      <Table>
        <TableHead>
          <TableRow>
            <TableCell></TableCell>
            <TableCell>Title</TableCell>
            <TableCell>Author</TableCell>
            <TableCell>Status</TableCell>
            <TableCell>Created At</TableCell>
            <TableCell>Comment Count</TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {reversedPullRequests.map((data) => {
            const node = data?.node;
            if (node == null) return null;
            return (
              <TableRow key={node.id}>
                <TableCell>
                  <small>
                    {node.pull_request_link({
                      children: node.number,
                      setRoute: props.setRoute,
                    })}
                  </small>
                </TableCell>
                <TableCell>{node.title}</TableCell>
                <TableCell>
                  {node.author?.user_link({
                    setRoute: props.setRoute,
                    children: node.author?.login,
                  })}
                </TableCell>
                <TableCell>{node.closed ? "Closed" : "Open"}</TableCell>
                <TableCell>{node.created_at_formatted}</TableCell>
                <TableCell>{node.totalCommentsCount}</TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </>
  );
}
