import * as React from "react";
import { bDeclare, read } from "@boulton/react";

export const billing_details_component = bDeclare`
  BillingDetails.billing_details_component @component {
    id,
    card_brand,
    credit_card_number,
    expiration_date,
    address,
    last_four_digits,
  }
`(function BillingDetailsComponent({
  data,
  setStateToFalse,
  ...additionalRuntimeProps /* unused here */
}) {
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
});
