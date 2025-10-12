import React from 'react';
import { iso } from '@iso';
import { ErrorBoundary } from './ErrorBoundary';
import { FullPageLoading } from './routes';
import { FragmentRenderer, useLazyReference } from '@isograph/react';

export function SmartestPetLoader() {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.SmartestPetRoute`),
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
