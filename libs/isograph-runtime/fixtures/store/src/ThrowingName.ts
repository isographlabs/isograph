import { iso } from './__isograph/iso';

export const ThrowingName = iso(`
  field Pet.ThrowingName {
    name
  }
`)(function ThrowingName({ data }) {
  if (data.name === 'Throw') {
    throw new Error('ThrowingName read the name Throw');
  }
  return data.name;
});
