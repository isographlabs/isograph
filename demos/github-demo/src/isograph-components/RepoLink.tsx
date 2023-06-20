const commit = "1a8182355ac9d121318213d1d8591480d5c1da12";

import { Alert, Link } from "@mui/material";

export const RepoLink = ({
  children,
  filePath,
}: {
  children: string;
  filePath: string;
}) => {
  return (
    <Alert severity="info" style={{ marginTop: 20 }}>
      Find the source code for the{" "}
      <Link
        href={`https://www.github.com/isographlabs/isograph/blob/${commit}/${filePath}`}
      >
        {children}
      </Link>
    </Alert>
  );
};
