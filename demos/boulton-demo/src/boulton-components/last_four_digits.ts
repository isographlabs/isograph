import { bDeclare } from "@boulton/react";
import type { ResolverParameterType } from "./__boulton/BillingDetails__last_four_digits.boulton";

export const last_four_digits = bDeclare<
  ResolverParameterType,
  ReturnType<typeof LastFour>
>`
  BillingDetails.last_four_digits @eager {
    credit_card_number,
  }
`(LastFour);

function LastFour(data: ResolverParameterType) {
  return data.credit_card_number.substring(data.credit_card_number.length - 4);
}
