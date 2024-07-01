import { iso } from '@iso';

export const Foo = iso(`
  field Pet.Foo {
    name
    tagline
  }
`)((data) => {
  return data;
});

export const Bar = iso(`
  field Query.LoadableDemo {
    pet(id: 0) {
      tagline
      Foo @loadable
    }
  }
`)((data) => {
  return data;
});
