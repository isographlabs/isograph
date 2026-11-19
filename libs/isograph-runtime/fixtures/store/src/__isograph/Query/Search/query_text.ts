export default 'query Search($q: String) {\n  search____filter___o_minAge__l_2_nameContains__v_q_c: search(filter: { minAge: 2, nameContains: $q }) {\n    __typename\n    id\n    name\n  }\n}';
