import { FullPageLoading } from '@/src/components/routes';
import { SmartestPetLoader } from '@/src/components/SmartestPetLoader';
import ThemeProvider from '@/src/theme';
import Head from 'next/head';
import { Suspense } from 'react';

export default function Smartest() {
  return (
    <>
      <Head>
        <title>Robert&apos;s Smartest Pet</title>
      </Head>
      <ThemeProvider>
        <Suspense fallback={<FullPageLoading />}>
          <SmartestPetLoader />
        </Suspense>
      </ThemeProvider>
    </>
  );
}
