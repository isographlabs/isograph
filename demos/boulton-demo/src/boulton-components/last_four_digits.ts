import { bDeclare } from "@boulton/react";

// TODO compiler will gnerate this type
type GeneratedType = {
  credit_card_number: string;
};

export const last_four_digits = bDeclare`
  BillingDetails.last_four_digits @eager {
    credit_card_number,
  }
`((data: GeneratedType) => {
  return data.credit_card_number.substring(data.credit_card_number.length - 4);
});
