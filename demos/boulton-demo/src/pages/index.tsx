import Head from "next/head";

import { BoultonDemo } from "@/boulton-components/BoultonDemo";

export default function Home() {
  return (
    <>
      <Head>
        <title>Boulton Demo</title>
        <meta
          name="description"
          content="Demonstration of network requests made lazily (i.e. during render) using react-disposable-state"
        />
      </Head>
      <div className="container">
        <BoultonDemo />
      </div>
    </>
  );
}
