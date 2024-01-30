import { iso } from '@isograph/react';
import { ResolverParameterType as AvatarProps } from '@iso/User/Avatar/reader.isograph';
import { Avatar as MuiAvatar } from '@mui/material';

export const Avatar = iso<AvatarProps, ReturnType<typeof AvatarComponent>>`
  field User.Avatar @component {
    name,
    avatarUrl,
  }
`(AvatarComponent);

function AvatarComponent(props: AvatarProps) {
  return <MuiAvatar alt={props.data.name ?? ''} src={props.data.avatarUrl} />;
}
