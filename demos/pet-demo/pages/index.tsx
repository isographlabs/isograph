import Head from 'next/head';
import { GraphQLConfDemo } from '@/src/components/router';
import ThemeProvider from '@/src/theme';

export default function Home() {
  return (
    <>
      <Head>
        <title>Robert&apos;s Pet List 3000</title>
      </Head>
      <ThemeProvider>
        <GraphQLConfDemo initialState={{ kind: 'Home' }} />
      </ThemeProvider>
    </>
  );
}
