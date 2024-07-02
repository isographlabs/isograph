import { iso } from '@iso';

export const Foo = iso(`
  field Pet.LoadableField {
    name
    tagline
  }
`)((data) => {
  console.log('LoadableField', data);
  return data;
});

export const Bar = iso(`
  field Query.LoadableDemo {
    pet(id: 0) {
      tagline
      LoadableField @loadable
    }
  }
`)((data) => {
  return data;
});
