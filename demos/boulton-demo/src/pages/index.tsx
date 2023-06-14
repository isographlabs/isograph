import Head from "next/head";

import { IsographDemo } from "@/isograph-componentsIsographnDemo";

// Initial Isograph setup. This will eventually happen when constructing an EnvironmentProvider
// or the like.
import { setNetwork } from "@isograph/react";
import { network } from "../network";
setNetwork(network);

export default function Home() {
  return (
    <>
      <Head>
        <title>Isograph Demo</title>
        <meta
          name="description"
          content="Demonstration of network requests made lazily (i.e. during render) using react-disposable-state"
        />
      </Head>
      <div className="container">
        <IsographDemo />
      </div>
    </>
  );
}
