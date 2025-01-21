import { iso } from '@iso';
import { Button } from '@mui/material';
import { Route } from './GithubDemo';
import { RepoGitHubLink } from './RepoGitHubLink';

export const HomePageList = iso(`
  field Query.HomePageList @component {
    viewer {
      login
      name
      RepositoryList
      __refetch
    }
  }
`)(function HomePageListComponent(
  { data },
  {
    setRoute,
  }: {
    setRoute: (route: Route) => void;
  },
) {
  return (
    <>
      <RepoGitHubLink filePath="demos/github-demo/src/isograph-components/HomePageList.tsx">
        Home Page List Component
      </RepoGitHubLink>
      <h1>
        {data.viewer.name}&apos;s ({data.viewer.login}) repository stats
      </h1>
      <Button onClick={() => data.viewer.__refetch()[1]()} variant="contained">
        Refetch viewer
      </Button>
      <data.viewer.RepositoryList setRoute={setRoute} />
    </>
  );
});
