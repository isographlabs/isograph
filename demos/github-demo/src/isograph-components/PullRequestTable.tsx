import { iso } from '@iso';

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
} from '@mui/material';
import { Route } from './GithubDemo';

export const createdAtFormatted = iso(`
  field PullRequest.createdAtFormatted {
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

export const PullRequestTable = iso(`
  field PullRequestConnection.PullRequestTable @component {
    edges {
      node {
        id
        PullRequestLink
        number
        title
        author {
          UserLink
          login
        }
        closed
        totalCommentsCount
        createdAtFormatted
      }
    }
  }
`)(function PullRequestTableComponent(
  { data },
  {
    setRoute,
  }: {
    setRoute: (route: Route) => void;
  },
) {
  const reversedPullRequests = [...(data.edges ?? [])].reverse();
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
                    <node.PullRequestLink setRoute={setRoute}>
                      {node.number}
                    </node.PullRequestLink>
                  </small>
                </TableCell>
                <TableCell>{node.title}</TableCell>
                <TableCell>
                  <author.UserLink setRoute={setRoute}>
                    {node.author?.login}
                  </author.UserLink>
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
});
