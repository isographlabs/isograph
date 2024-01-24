import clsx from "clsx";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
import Layout from "@theme/Layout";
import Heading from "@theme/Heading";

import styles from "./index.module.css";
import HomepageFeatures from "../components/HomePageFeatures";
import Link from "@docusaurus/Link";

function HomepageHeader() {
  const { siteConfig } = useDocusaurusContext();
  return (
    <header className={clsx("hero hero--primary", styles.heroBanner)}>
      <div className="container">
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div style={{ display: "flex", justifyContent: "center", gap: "2em" }}>
          <div className={styles.buttons}>
            <Link
              className="button button--secondary button--lg"
              to="/docs/quickstart"
            >
              Quickstart
            </Link>
          </div>
        </div>
      </div>
    </header>
  );
}

export default function Home(): JSX.Element {
  return (
    <Layout
      title={`Isograph — select your components like you select your fields: with GraphQL`}
      description="Isograph, the framework for building data-driven apps"
    >
      <HomepageHeader />
      <main>
        <HomepageFeatures />
      </main>
    </Layout>
  );
}
