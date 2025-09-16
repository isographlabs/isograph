import { RandomLoader } from '@/src/components/RandomLoader';
import { FullPageLoading } from '@/src/components/routes';
import ThemeProvider from '@/src/theme';
import Head from 'next/head';
import { Suspense, useEffect, useState } from 'react';

export default function Random() {
  // Note: there is a bug in which the page suspends indefinitely, due to
  // the pointer network response not triggering a subscription. This allows us
  // to see the loading state and investigate on the client.
  const [isMounted, setIsMounted] = useState(false);
  useEffect(() => setIsMounted(true), []);

  return (
    <>
      <Head>
        <title>Miscellaneous page to test random stuff</title>
      </Head>
      <ThemeProvider>
        <Suspense fallback={<FullPageLoading />}>
          {isMounted && <RandomLoader />}
        </Suspense>
      </ThemeProvider>
    </>
  );
}
