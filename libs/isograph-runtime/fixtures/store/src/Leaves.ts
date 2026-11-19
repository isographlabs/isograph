import { iso } from './__isograph/iso';

export const Leaves = iso(`
  field Query.Leaves($name: String) {
    greeting
    literalGreeting: greeting(name: "Ada Lovelace", loud: true, times: 3)
    nullGreeting: greeting(name: null)
    variableGreeting: greeting(name: $name)
    tags
    settings
  }
`)(function Leaves({ data }) {
  return data;
});

iso(`entrypoint Query.Leaves`);
