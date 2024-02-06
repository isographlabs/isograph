import { iso } from '@iso';
import { Avatar as MuiAvatar } from '@mui/material';

export const Avatar = iso(`
  field User.Avatar @component {
    name,
    avatarUrl,
  }
`)(function AvatarComponent(props) {
  return <MuiAvatar alt={props.data.name ?? ''} src={props.data.avatarUrl} />;
});
