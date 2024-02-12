import Layout from '@theme/Layout';

import YoutubeEmbed from '../components/YoutubeEmbed';
import ProblemStatement from '../components/ProblemStatement';
import IsographFeatures from '../components/IsographFeatures';
import HomepageHeader from '../components/Header';
import Components from '../components/Components';
import Fetching from '../components/Fetching';
import IsIsographRightForMe from '../components/IsIsographRightForMe';
import IntroducingIsograph from '../components/IntroducingIsograph';

export default function Home(): JSX.Element {
  return (
    <Layout
      title={`Isograph — select your components like you select your fields: with GraphQL`}
      description="Isograph, the framework for building data-driven apps"
    >
      <HomepageHeader />
      <main>
        <ProblemStatement />
        <IntroducingIsograph />
        <IsographFeatures />
        <YoutubeEmbed />
        <Components />
        <Fetching />
        <IsIsographRightForMe />
      </main>
    </Layout>
  );
}
