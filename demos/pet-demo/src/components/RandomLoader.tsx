import React from 'react';
import { iso } from './__isograph/iso';
import { ErrorBoundary } from './ErrorBoundary';
import { FullPageLoading } from './routes';
import {
  FragmentRenderer,
  LoadableFieldReader,
  useClientSideDefer,
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
        name
      }
    }
  }
`)(({ data }) => {
  return (
    <div>
      random page
      <br />
      <React.Suspense fallback={<FullPageLoading />}>
        <LoadableFieldReader loadableField={data.firstNode} args={{}}>
          {(firstNode) => {
            return JSON.stringify(firstNode, null, 2);
          }}
        </LoadableFieldReader>
      </React.Suspense>
    </div>
  );
});
