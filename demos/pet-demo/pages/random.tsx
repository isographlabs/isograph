import { RandomLoader } from '@/src/components/RandomLoader';
import { FullPageLoading } from '@/src/components/routes';
import ThemeProvider from '@/src/theme';
import Head from 'next/head';
import { Suspense } from 'react';

export default function Smartest() {
  return (
    <>
      <Head>
        <title>Miscellaneous page to test random stuff</title>
      </Head>
      <ThemeProvider>
        <Suspense fallback={<FullPageLoading />}>
          <RandomLoader />
        </Suspense>
      </ThemeProvider>
    </>
  );
}
