import React from 'react';
import { iso } from './__isograph/iso';
import { ErrorBoundary } from './ErrorBoundary';
import { FullPageLoading } from './routes';
import {
  FragmentRenderer,
  LoadableFieldReader,
  useLazyReference,
} from '@isograph/react';

export function RandomLoader() {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.Random`),
    {},
  );

  return (
    <ErrorBoundary>
      <React.Suspense fallback={<FullPageLoading />}>
        <FragmentRenderer
          fragmentReference={fragmentReference}
          networkRequestOptions={{ suspendIfInFlight: false }}
        />
      </React.Suspense>
    </ErrorBoundary>
  );
}

export const Random = iso(`
  field Query.Random @component {
    firstNode {
      asPet {
        nickname
      }
    }
  }
`)(({ data }) => {
  return (
    <div>
      random page
      <br />
      <React.Suspense fallback={'loading firstNode'}>
        <LoadableFieldReader loadableField={data.firstNode} args={{}}>
          {(firstNode) => {
            console.log('firstNode', { firstNode });
            return JSON.stringify(firstNode, null, 2);
          }}
        </LoadableFieldReader>
      </React.Suspense>
    </div>
  );
});
