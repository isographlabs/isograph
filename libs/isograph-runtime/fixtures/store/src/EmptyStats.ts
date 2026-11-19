import { iso } from './__isograph/iso';

export const EmptyStats = iso(`
  field Query.EmptyStats($id: ID!) {
    pet(id: $id) {
      stats {
      }
    }
  }
`)(function EmptyStats({ data }) {
  return data;
});

iso(`entrypoint Query.EmptyStats`);
