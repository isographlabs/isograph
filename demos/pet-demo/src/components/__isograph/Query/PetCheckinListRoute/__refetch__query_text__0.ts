export default 'mutation Query__make_super($checkin_id: ID!) {\
  make_checkin_super____checkin_id___v_checkin_id: make_checkin_super(checkin_id: $checkin_id) {\
    icheckin {\
      ... on Checkin {\
        __typename,\
        id,\
        location,\
      },\
    },\
  },\
}';