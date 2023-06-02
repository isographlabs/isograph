import * as React from "react";
import { bDeclare } from "@boulton/react";

export const billing_details_component = bDeclare`
  BillingDetails.billing_details_component {
    id,
    card_brand,
    credit_card_number,
    expiration_date,
    address,
  }
`((data) => (
  <>
    <h2>Billing details</h2>
    <p>Card number: {data.credit_card_number}</p>
    <p>Expiration date: {data.expiration_date}</p>
    <p>Card type: {data.card_brand}</p>
    <p>Address: {data.address}</p>
  </>
));
