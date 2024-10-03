import { iso } from '@iso';

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
} from '@mui/material';
import { Route } from './GithubDemo';

export const RepositoryList = iso(`
  field User.RepositoryList @component {
    repositories(last: 10) {
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
`)(function UserRepositoryListComponent(
  { data },
  { setRoute }: { setRoute: (route: Route) => void },
) {
  const repositories = [...(data.repositories.edges ?? [])].reverse();
  return (
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
  );
});
