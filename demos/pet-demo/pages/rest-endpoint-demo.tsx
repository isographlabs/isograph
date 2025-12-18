import { RestEndpointDemoLoader } from '@/src/components/RestEndpointDemoRoute';
import { FullPageLoading } from '@/src/components/routes';
import ThemeProvider from '@/src/theme';
import Head from 'next/head';
import { useRouter } from 'next/router';
import { Suspense } from 'react';

export default function RestEndpoingDemoPage() {
  const router = useRouter();

  return (
    <>
      <Head>
        <title>Rest Endpoint Demo</title>
      </Head>
      <ThemeProvider>
        <Suspense fallback={<FullPageLoading />}>
          <RestEndpointDemoLoader />
        </Suspense>
      </ThemeProvider>
    </>
  );
}
