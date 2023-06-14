import * as React from "react";
import { bDeclare, type IsographComponentProps } from "@isograph/react";

import type { ResolverParameterType } from "./__isograph/BillingDetails__billing_details_component.isograph";

export const billing_details_component = bDeclare<
  ResolverParameterType,
  ReturnType<typeof BillingDetailsComponent>
>`
  BillingDetails.billing_details_component @component {
    id,
    card_brand,
    credit_card_number,
    expiration_date,
    address,
    last_four_digits,
  }
`(BillingDetailsComponent);

function BillingDetailsComponent({
  data,
  setStateToFalse,
}: ResolverParameterType) {
  const [showFullCardNumber, setShowFullCardNumber] = React.useState(false);

  return (
    <>
      <h2>Billing details</h2>
      {showFullCardNumber ? (
        <p>Card number: {data.credit_card_number}</p>
      ) : (
        <p onClick={() => setShowFullCardNumber(true)}>
          Card number: ...{data.last_four_digits}
        </p>
      )}
      <p>Expiration date: {data.expiration_date}</p>
      <p onClick={setStateToFalse}>Card type: {data.card_brand}</p>
      <p>Address: {data.address}</p>
    </>
  );
}
