import { iso } from "@isograph/react";
import { ResolverParameterType as AvatarProps } from "@iso/User/avatar/reader.isograph";
import { Avatar } from "@mui/material";

export const avatar = iso<AvatarProps, ReturnType<typeof Avatar>>`
  User.avatar @component {
    name,
    avatarUrl,
  }
`(AvatarComponent);

function AvatarComponent(props: AvatarProps) {
  return <Avatar alt={props.data.name ?? ""} src={props.data.avatarUrl} />;
}
