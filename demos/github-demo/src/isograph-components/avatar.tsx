import { iso } from "@isograph/react";
import { ResolverParameterType as AvatarProps } from "./__isograph/User/avatar.isograph";
import { Avatar, Box } from "@mui/material";

export const avatar = iso<AvatarProps, ReturnType<typeof Avatar>>`
  User.avatar @component {
    name,
    avatarUrl,
  }
`(AvatarComponent);

function AvatarComponent(props: AvatarProps) {
  return <Avatar alt={props.data.name ?? ""} src={props.data.avatarUrl} />;
}
