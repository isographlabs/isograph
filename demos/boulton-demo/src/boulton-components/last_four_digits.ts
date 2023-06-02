import { bDeclare } from "@boulton/react";

export const last_four_digits = bDeclare`
  BillingDetails.last_four_digits @eager {
    credit_card_number,
  }
`((data) => {
  return data.credit_card_number.substring(data.credit_card_number.length - 4);
});
