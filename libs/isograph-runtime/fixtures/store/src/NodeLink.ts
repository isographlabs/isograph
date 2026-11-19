import { iso } from './__isograph/iso';

export const NodeLink = iso(`
  field Node.NodeLink
`)(function NodeLink({ link }) {
  return link;
});
