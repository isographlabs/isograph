import { render } from 'react-dom';
import { createTheme, ThemeProvider } from "@mui/material/styles";
import { CssBaseline } from '@mui/material';

import { GithubDemo } from "@/isograph-components/github_demo";
import { setNetwork } from "@isograph/react";

function makeNetworkRequest<T>(queryText: string, variables: any): Promise<T> {
  let promise = fetch("https://api.github.com/graphql", {
    method: "POST",
    headers: {
      Authorization: "Bearer " + process.env.NEXT_PUBLIC_GITHUB_TOKEN,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ query: queryText, variables }),
  }).then((response) => response.json());
  return promise;
}
setNetwork(makeNetworkRequest);


const theme = createTheme({
  palette: {
    primary: {
      light: "#788caf",
      main: "#385276",
      dark: "#1a2f4a",
      contrastText: "#fff",
    },
    secondary: {
      light: "#ff7961",
      main: "#f28800",
      dark: "#e86600",
      contrastText: "#000",
    },
  },
});

render(
	<>
		<CssBaseline />
		<ThemeProvider theme={theme}>
			<GithubDemo />
		</ThemeProvider>
	</>,
	document.getElementById("root")!
)
