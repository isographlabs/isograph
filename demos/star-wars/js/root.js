

import {bDeclare, useLazyLoadQuery, read} from '@boulton/react';
import * as React from 'react';


export const root = bDeclare`
  # this should probably have an @fetchable directive or something,
  # otherwise, every component on Query gets a normalization AST,
  # whether or not it will be used. (Only the root needs it.)
  Query.root { header_component, current_post_component, }
`(data => {
  return <>
    {data.header}
    {data.current_post_component}
  </>
});

function RootComponent() {
  const queryRef = useLazyQueryRef(root, {/* variables */}, { /* options */});
  return <Suspense fallback="loading">
      <RefReader ref={queryRef} />
    </Suspense>
}
function RefReader({queryRef}) {
  return read(queryRef);
}