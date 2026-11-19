import { iso } from './__isograph/iso';

export const StatsLink = iso(`
  field PetStats.StatsLink
`)(function StatsLink({ link }) {
  return link;
});
