import { iso } from '@iso';

import { usePagination } from '@isograph/react';
import {
  Button,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
} from '@mui/material';
import type { ReactNode } from 'react';
import { Route } from './GithubDemo';

export const RepositoryList = iso(`
  field User.RepositoryList @component {
    RepositoryConnection @loadable
  }
`)(function UserRepositoryListComponent(
  { data },
  {
    setRoute,
    children,
  }: { children?: ReactNode; setRoute: (route: Route) => void },
) {
  const pagination = usePagination(data.RepositoryConnection);
  const repositories = [...pagination.results].reverse();
  return (
    <>
      <Button
        variant="contained"
        style={{ marginInlineEnd: '1rem' }}
        disabled={!(pagination.kind === 'Complete' && pagination.hasNextPage)}
        onClick={() =>
          pagination.kind === 'Complete' &&
          pagination.fetchMore(undefined, 10)
        }
      >
        {pagination.kind === 'Complete' && !pagination.hasNextPage
          ? 'All fetched'
          : 'Fetch more'}
      </Button>

      {children}
      <Table>
        <TableHead>
          <TableRow>
            <TableCell>Repository</TableCell>
            <TableCell>Stars</TableCell>
            <TableCell>Forks</TableCell>
            <TableCell>Total PRs</TableCell>
            <TableCell>Watchers</TableCell>
          </TableRow>
        </TableHead>
        <TableBody>
          {repositories.map((data) => {
            if (data == null || data.node == null) {
              return null;
            }
            const { node } = data;
            return (
              <TableRow key={node.id}>
                <TableCell>
                  <node.RepositoryLink setRoute={setRoute}>
                    {node.nameWithOwner}
                  </node.RepositoryLink>
                </TableCell>
                <TableCell>{node.stargazerCount}</TableCell>
                <TableCell>{node.forkCount}</TableCell>
                <TableCell>{node.pullRequests?.totalCount}</TableCell>
                <TableCell>{node.watchers?.totalCount}</TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </>
  );
});

export const RepositoryConnection = iso(`
  field User.RepositoryConnection($first: Int, $after: String) {
    repositories(last: $first, before: $after) {
      pageInfo {
        hasNextPage: hasPreviousPage
        endCursor: startCursor
      }
      edges {
        node {
          id
          RepositoryLink
          name
          nameWithOwner
          description
          forkCount
          pullRequests {
            totalCount
          }
          stargazerCount
          watchers {
            totalCount
          }
        }
      }
    }
  }
`)(function UserRepositoryConnectionComponent({ data }) {
  return data.repositories;
});
