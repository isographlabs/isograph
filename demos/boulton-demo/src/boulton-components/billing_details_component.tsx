import * as React from "react";
import { bDeclare, type BoultonComponentProps } from "@boulton/react";

// TODO the compiler will generate this type
type GeneratedDataType = {
  id: string;
  card_brand: string;
  credit_card_number: string;
  expiration_date: string;
  address: string;
  last_four_digits: string;
};

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
}: BoultonComponentProps<GeneratedDataType, { setStateToFalse: () => void }>) {
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
