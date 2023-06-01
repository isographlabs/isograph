import React from "react";
import NoSSR from "react-no-ssr";

import queryHello from "../boulton-components/__boulton/Query__hello_component.boulton";
import { useLazyReference, read } from "@boulton/react";

export function LazyLoadPostsWrapper() {
  return (
    <NoSSR>
      <React.Suspense fallback={<FullPageLoading />}>
        <PostsLazyLoader />
      </React.Suspense>
    </NoSSR>
  );
}

function FullPageLoading() {
  return <h1 className="mt-5">Loading...</h1>;
}

function PostsLazyLoader() {
  const { queryReference } = useLazyReference(queryHello);
  const data = read(queryReference);
  console.log("data", data);
  return <div>{JSON.stringify(data)}</div>;
}
