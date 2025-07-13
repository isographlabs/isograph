import { iso } from '@iso';

export const foo = iso(`
  field Query.Foo($id: ID!) {
    pet(id: $id) {
      name
    }
  }
`)(() => {});
