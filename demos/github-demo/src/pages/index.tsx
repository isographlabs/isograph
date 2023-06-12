import Head from "next/head";

import { GithubDemo } from "@/boulton-components/github_demo";
import { setNetwork } from "@boulton/react";

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

import { createTheme, ThemeProvider } from "@mui/material/styles";

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

export default function Home() {
  return (
    <>
      <Head>
        <title>Github Demo</title>
        <meta
          name="description"
          content="Demonstration of Isograph, used with Github's GraphQL API."
        />
      </Head>
      <ThemeProvider theme={theme}>
        <GithubDemo />
      </ThemeProvider>
    </>
  );
}
