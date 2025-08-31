import { FullPageLoading } from '@/src/components/routes';
import { SmartestPetLoader } from '@/src/components/SmartestPetLoader';
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
        <title>Robert&apos;s Smartest Pet</title>
      </Head>
      <ThemeProvider>
        <Suspense fallback={<FullPageLoading />}>
          {isMounted && <SmartestPetLoader />}
        </Suspense>
      </ThemeProvider>
    </>
  );
}
