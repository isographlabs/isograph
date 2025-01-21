import { LazyLoadPostsWrapper } from '@/components/LazyLoadPostsPage';
import Head from 'next/head';

export default function Home() {
  return (
    <>
      <Head>
        <title>Lazy loading demonstration</title>
        <meta
          name="description"
          content="Demonstration of network requests made lazily (i.e. during render) using react-disposable-state"
        />
      </Head>
      <div className="container">
        <LazyLoadPostsWrapper />
      </div>
    </>
  );
}
