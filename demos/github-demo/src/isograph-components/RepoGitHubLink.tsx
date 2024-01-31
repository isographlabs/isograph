import { Alert, Link } from '@mui/material';

export const RepoGitHubLink = ({
  children,
  filePath,
}: {
  children: string;
  filePath: string;
}) => {
  return (
    <Alert severity="info" style={{ marginTop: 20 }}>
      Find the source code for the{' '}
      <Link
        href={`https://www.github.com/isographlabs/isograph/blob/main/${filePath}`}
      >
        {children}
      </Link>
    </Alert>
  );
};
