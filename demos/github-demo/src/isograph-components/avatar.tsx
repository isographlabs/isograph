import { iso } from '@iso';
import { ResolverParameterType as AvatarProps } from '@iso/User/Avatar/reader';
import { Avatar as MuiAvatar } from '@mui/material';

export const Avatar = iso<AvatarProps>`
  field User.Avatar @component {
    name,
    avatarUrl,
  }
`(AvatarComponent);

function AvatarComponent(props: AvatarProps) {
  return <MuiAvatar alt={props.data.name ?? ''} src={props.data.avatarUrl} />;
}
