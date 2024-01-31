import { Html, Head, Main, NextScript } from 'next/document';

import { CssBaseline } from '@mui/material';

export default function Document() {
  return (
    <Html lang="en" data-bs-theme="dark">
      <Head>
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <meta name="color-scheme" content="dark" />
        <link
          rel="stylesheet"
          href="https://fonts.googleapis.com/css?family=Roboto:300,400,500,700&display=swap"
        />
        <link
          rel="stylesheet"
          href="https://fonts.googleapis.com/icon?family=Material+Icons"
        />
      </Head>
      <body>
        <CssBaseline />
        <Main />
        <NextScript />
      </body>
    </Html>
  );
}
