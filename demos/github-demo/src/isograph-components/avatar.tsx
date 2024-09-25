import { iso } from '@iso';
import { Avatar as MuiAvatar } from '@mui/material';

export const Avatar = iso(`
  field User.Avatar @component {
    name
    avatarUrl
  }
`)(function AvatarComponent({ data }) {
  return <MuiAvatar alt={data.name ?? ''} src={data.avatarUrl} />;
});
