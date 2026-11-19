import { iso } from './__isograph/iso';

export const Search = iso(`
  field Query.Search($q: String) {
    search(filter: { nameContains: $q, minAge: 2 }) {
      id
      name
    }
  }
`)(function Search({ data }) {
  return data;
});

iso(`entrypoint Query.Search`);
