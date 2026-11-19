import { iso } from './__isograph/iso';

export const NestedQuery = iso(`
  field Query.NestedQuery {
    greeting
    query {
      tags
    }
  }
`)(function NestedQuery({ data }) {
  return data;
});

iso(`entrypoint Query.NestedQuery`);
