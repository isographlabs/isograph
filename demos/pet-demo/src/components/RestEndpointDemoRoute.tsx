import { Container } from '@mui/material';
import { iso } from '@iso';
import {
  UNASSIGNED_STATE,
  useUpdatableDisposableState,
} from '@isograph/react-disposable-state';
import { Suspense, useEffect } from 'react';
import {
  FragmentReferenceOfEntrypoint,
  FragmentRenderer,
  useIsographEnvironment,
  writeData,
} from '@isograph/react';
import { type Query__PetNameList__raw_response_type } from './__isograph/Query/PetNameList/raw_response_type';

export function RestEndpointDemoLoader() {
  const entrypoint = iso(`entrypoint Query.PetNameList`);

  const { state, setState } =
    useUpdatableDisposableState<
      FragmentReferenceOfEntrypoint<typeof entrypoint>
    >();

  const environment = useIsographEnvironment();
  useEffect(() => {
    makeRestRequest().then((restData) => {
      setState(writeData(environment, entrypoint, restData, {}));
    });
  }, []);

  return (
    <Container maxWidth="md">
      <h1>Rest Endpoint Demo</h1>

      {state === UNASSIGNED_STATE ? (
        <div>Loading pets...</div>
      ) : (
        <Suspense fallback={<div>Loading pets...</div>}>
          <FragmentRenderer fragmentReference={state} additionalProps={{}} />
        </Suspense>
      )}
    </Container>
  );
}

function makeRestRequest() {
  const { resolve, promise } =
    Promise.withResolvers<Query__PetNameList__raw_response_type>();
  setTimeout(() => {
    resolve({
      pets: [
        {
          id: '0',
          firstName: 'Mr BigglesWorth',
          lastName: 'Evil',
        },
        {
          id: '1',
          firstName: 'Scooby',
          lastName: 'Doo',
        },
        {
          id: '2',
          firstName: 'Spike',
          lastName: 'Pickles',
        },
      ],
    });
  }, 2000);

  return promise;
}

export const PetNameList = iso(`
  field Query.PetNameList @component {
    pets {
      id
      fullName
    }
  }
`)(({ data }) => {
  return data.pets.map((pet) => {
    return <h2 key={pet.id}>{pet.fullName}</h2>;
  });
});
