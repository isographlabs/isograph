import { iso } from '@iso';

import { useConnectionSpecPagination } from '@isograph/react';
import {
  Button,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
} from '@mui/material';
import { Route } from './GithubDemo';

export const RepositoryList = iso(`
  field User.RepositoryList @component {
    firstPage: RepositoryConnection(first: 10)
    RepositoryConnection @loadable
  }
`)(function UserRepositoryListComponent(
  { data },
  { setRoute }: { setRoute: (route: Route) => void },
) {
  const pagination = useConnectionSpecPagination(
    data.RepositoryConnection,
    data.firstPage.pageInfo,
  );
  const repositories = (data.firstPage.edges ?? []).concat(pagination.results);
  return (
    <>
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
          <TableRow>
            <TableCell>
              <Button
                variant="contained"
                disabled={
                  !(pagination.kind === 'Complete' && pagination.hasNextPage)
                }
                onClick={() =>
                  pagination.kind === 'Complete' && pagination.fetchMore(10)
                }
              >
                {pagination.kind === 'Complete' && !pagination.hasNextPage
                  ? 'All fetched'
                  : 'Fetch more'}
              </Button>
            </TableCell>
            <TableCell></TableCell>
            <TableCell></TableCell>
            <TableCell></TableCell>
            <TableCell></TableCell>
          </TableRow>
        </TableBody>
      </Table>
    </>
  );
});

export const RepositoryConnection = iso(`
  field User.RepositoryConnection($first: Int, $after: String) {
    repositories(first: $first, after: $after) {
      pageInfo {
        hasNextPage
        endCursor
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
