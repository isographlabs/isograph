import { iso } from '@iso';

export const Foo = iso(`
  field Pet.Foo {
    name
  }
`)((data) => {
  return data;
});

export const Bar = iso(`
  field Query.LoadableDemo {
    pet(id: 0) {
      name
      Foo @loadable
    }
  }
`)((data) => {
  return data;
});
