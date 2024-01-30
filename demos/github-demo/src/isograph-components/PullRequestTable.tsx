import { iso } from '@isograph/react';
import type { ResolverParameterType as PullRequestTableParams } from '@iso/PullRequestConnection/PullRequestTable/reader.isograph';
import type { ResolverParameterType as CreatedAtFormattedType } from '@iso/PullRequest/createdAtFormatted/reader.isograph';

import { Table, TableBody, TableCell, TableHead, TableRow } from '@mui/material';

export const createdAtFormatted = iso<CreatedAtFormattedType>`
  field PullRequest.createdAtFormatted {
    createdAt,
  }
`((props) => {
  const date = new Date(props.createdAt);
  return date.toLocaleDateString('en-us', {
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
  });
});

export const PullRequestTable = iso<PullRequestTableParams>`
  field PullRequestConnection.PullRequestTable @component {
    edges {
      node {
        id,
        PullRequestLink,
        number,
        title,
        author {
          UserLink,
          login,
        },
        closed,
        totalCommentsCount,
        createdAtFormatted,
      },
    },
  }
`(PullRequestTableComponent);

function PullRequestTableComponent(props: PullRequestTableParams) {
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
            const author = node.author;
            if (author == null) return null;
            return (
              <TableRow key={node.id}>
                <TableCell>
                  <small>
                    <node.PullRequestLink setRoute={props.setRoute}>
                      {node.number}
                    </node.PullRequestLink>
                  </small>
                </TableCell>
                <TableCell>{node.title}</TableCell>
                <TableCell>
                  <author.UserLink setRoute={props.setRoute}>{node.author?.login}</author.UserLink>
                </TableCell>
                <TableCell>{node.closed ? 'Closed' : 'Open'}</TableCell>
                <TableCell>{node.createdAtFormatted}</TableCell>
                <TableCell>{node.totalCommentsCount}</TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </>
  );
}
