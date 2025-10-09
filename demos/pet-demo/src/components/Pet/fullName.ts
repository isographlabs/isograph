import { iso } from '@iso';

export const fullName = iso(`
  field Pet.fullName {
    firstName
    lastName
  }
`)(({ data }) => data.firstName + ' ' + data.lastName);
