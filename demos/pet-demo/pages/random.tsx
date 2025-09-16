import { RandomLoader } from '@/src/components/RandomLoader';
import { FullPageLoading } from '@/src/components/routes';
import ThemeProvider from '@/src/theme';
import Head from 'next/head';
import { Suspense, useEffect, useState } from 'react';

export default function Smartest() {
  const [isMounted, setIsMounted] = useState(false);
  useEffect(() => setIsMounted(true), []);
  console.log('route level');
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
