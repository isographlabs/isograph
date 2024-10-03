import React from 'react';
import { iso } from '@iso';
import { useLazyReference, useResult } from '@isograph/react';
import { PetByNameRoute } from './routes';

export const PetByNameRouteComponent = iso(`
  field Query.PetByName($name: String!) @component {
    pet: petByName(name: $name) {
      PetDetailDeferredRouteInnerComponent
    }
  }
`)(function ({ data }) {
  const { pet } = data;
  if (pet == null) {
    return <h1>Pet not found.</h1>;
  }

  return <pet.PetDetailDeferredRouteInnerComponent />;
});

export function PetByNameRouteLoader({ route }: { route: PetByNameRoute }) {
  const { fragmentReference } = useLazyReference(
    iso(`entrypoint Query.PetByName`),
    { name: route.name },
  );

  const Component = useResult(fragmentReference, {});
  return <Component />;
}
