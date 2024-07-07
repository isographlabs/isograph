import { iso } from '@iso';
import { EntrypointReader, useClientSideDefer } from '@isograph/react';
import { Suspense } from 'react';

export const Foo = iso(`
  field Pet.LoadableField @component {
    tagline
    alt_tagline
  }
`)((data) => {
  console.log('LoadableField', data);
  return (
    <>
      We deferred loading of the tagline, which is {data.tagline}. Alt:{' '}
      {data.alt_tagline}
    </>
  );
});

export const Bar = iso(`
  field Query.LoadableDemo @component {
    pet(id: 0) {
      name
      LoadableField @loadable
    }
  }
`)(({ pet }) => {
  if (pet == null) {
    return <>no pet</>;
  }

  console.log(pet.LoadableField);
  const data = useClientSideDefer(pet.LoadableField);
  console.log('deferred data', data);
  return (
    <>
      <h1>Name: {pet.name}</h1>
      <Suspense fallback="loading">
        <EntrypointReader queryReference={data} />
      </Suspense>
    </>
  );
});
