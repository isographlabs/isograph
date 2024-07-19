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

export const Foo2 = iso(`
  field Pet.LoadableField2 @component {
    tagline
    alt_tagline
  }
`)((data) => {
  console.log('LoadableField', data);
  return <>alt</>;
});

export const Bar = iso(`
  field Query.LoadableDemo @component {
    pet(id: 0) {
      name
      LoadableField @loadable
      LoadableField2 @loadable
    }
  }
`)(({ pet }) => {
  if (pet == null) {
    return <>no pet</>;
  }

  console.log('pet', pet);
  const data = useClientSideDefer(pet.LoadableField);
  const data2 = useClientSideDefer(pet.LoadableField2);
  console.log('deferred data', data);
  return (
    <>
      <h1>Name: {pet.name}</h1>
      <Suspense fallback="loading">
        <EntrypointReader queryReference={data} />
        <br />
        <EntrypointReader queryReference={data2} />
      </Suspense>
    </>
  );
});
