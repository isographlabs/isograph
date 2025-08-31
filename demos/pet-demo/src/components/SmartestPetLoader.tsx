import React from 'react';
import { iso } from './__isograph/iso';
import { ErrorBoundary } from './ErrorBoundary';
import { FullPageLoading } from './routes';
import { FragmentReader, useLazyReference } from '@isograph/react';

export function SmartestPetLoader() {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.SmartestPetRoute`),
    {},
  );

  return (
    <ErrorBoundary>
      <React.Suspense fallback={<FullPageLoading />}>
        <FragmentReader
          fragmentReference={fragmentReference}
          networkRequestOptions={{ suspendIfInFlight: false }}
        />
      </React.Suspense>
    </ErrorBoundary>
  );
}
