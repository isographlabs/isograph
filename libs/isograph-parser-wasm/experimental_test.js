import { parse_iso_literal, init_panic_hook } from './isograph_lang_parser_wasm.js';

// Initialize the WASM module
init_panic_hook();

// Test parsing a valid field
const validField = `field User.name @component {
  id
  name
}`;

console.log('Testing valid field:');
const result1 = parse_iso_literal(validField, 'User.ts', 'UserName');
console.log(JSON.parse(result1));

// Test parsing an entrypoint
const validEntrypoint = `entrypoint Query.user`;

console.log('\nTesting entrypoint:');
const result2 = parse_iso_literal(validEntrypoint, 'Query.ts', null);
console.log(JSON.parse(result2));

// Test parsing invalid syntax
const invalid = `field User.name { invalid syntax }`;

console.log('\nTesting invalid syntax:');
const result3 = parse_iso_literal(invalid, 'User.ts', null);
console.log(JSON.parse(result3));