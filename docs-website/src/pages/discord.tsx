import Layout from '@theme/Layout';
import { useEffect } from 'react';

export default function Home(): JSX.Element {
  useEffect(() => {
    // @ts-expect-error
    window.location = 'https://discord.gg/qcHUxb6deQ';
  }, []);
  return (
    <Layout
      title={`Isograph, the UI framework for teams that move fast without breaking things`}
      description="Isograph, the UI framework for teams that move fast without breaking things"
    >
      <main>
        <div
          style={{
            flexDirection: 'row',
            display: 'flex',
            justifyContent: 'center',
          }}
        >
          <h1 style={{ paddingTop: 50 }}>
            <a href="https://discord.gg/qcHUxb6deQ">
              Click here to join the Isograph discord
            </a>
          </h1>
        </div>
      </main>
    </Layout>
  );
}
