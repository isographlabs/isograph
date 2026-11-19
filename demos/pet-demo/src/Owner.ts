import { iso } from './__isograph/iso';
import type { ResolverParameters } from './iso';

iso(`entity Owner`);

export const displayName = iso(`
  field Owner.displayName "How the owner signs their name."
`)(function displayName() {
  return 'The owner';
});

export const OwnerBadge = iso(`
  field Owner.OwnerBadge @component {
    displayName
  }
`)(function OwnerBadge({ data }: ResolverParameters<{ displayName: string }>) {
  return `Owned by ${data.displayName}`;
});

export const owner = iso(`
  field Pet.owner to Owner! {
    id
  }
`)(function owner({ data }: ResolverParameters<{ id: string }>) {
  return `owner-of-${data.id}`;
});
