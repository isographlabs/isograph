import { bDeclare } from "@isograph/react";
import * as React from "react";

import type { ResolverParameterType } from "./__isograph/User__avatar_component.isograph";

export const avatar_component = bDeclare<ResolverParameterType, unknown>`
  "An avatar"
  User.avatar_component @component {
    name,
    email,
    avatar_url,
  }
`(Avatar);

function Avatar({ data }: ResolverParameterType) {
  return (
    <div>
      <a href={`mailto:${data.email}`}>
        Send email to {data.name}
        <img
          src={data.avatar_url}
          style={{ height: 100, width: 100 }}
          alt={`email ${data.name}`}
        />
      </a>
    </div>
  );
}
