import type {IsographEntrypoint} from '@isograph/react';
import { Query__meNameSuccessor__param } from './Query/meNameSuccessor/param_type';
import { Query__meName__param } from './Query/meName/param_type';
import { Query__nodeField__param } from './Query/nodeField/param_type';
import entrypoint_Query__meNameSuccessor from '../__isograph/Query/meNameSuccessor/entrypoint';
import entrypoint_Query__meName from '../__isograph/Query/meName/entrypoint';
import entrypoint_Query__nodeField from '../__isograph/Query/nodeField/entrypoint';

type IdentityWithParam<TParam> = <TClientFieldReturn>(
  x: (param: TParam) => TClientFieldReturn
) => (param: TParam) => TClientFieldReturn;
type IdentityWithParamComponent<TParam> = <TClientFieldReturn, TAdditionalProps = Record<string, never>>(
  x: (data: TParam, secondParam: TAdditionalProps) => TClientFieldReturn
) => (data: TParam, secondParam: TAdditionalProps) => TClientFieldReturn;

type WhitespaceCharacter = ' ' | '\t' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

// This is a recursive TypeScript type that matches strings that
// start with whitespace, followed by TString. So e.g. if we have
// ```
// export function iso<T>(
//   param: T & MatchesWhitespaceAndString<'field Query.foo', T>
// ): Bar;
// ```
// then, when you call
// ```
// const x = iso(`
//   field Query.foo ...
// `);
// ```
// then the type of `x` will be `Bar`, both in VSCode and when running
// tsc. This is how we achieve type safety — you can only use fields
// that you have explicitly selected.
type MatchesWhitespaceAndString<
  TString extends string,
  T
> = Whitespace<T> extends `${TString}${string}` ? T : never;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.meNameSuccessor', T>
): IdentityWithParam<Query__meNameSuccessor__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.meName', T>
): IdentityWithParam<Query__meName__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.nodeField', T>
): IdentityWithParam<Query__nodeField__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.meNameSuccessor', T>
): typeof entrypoint_Query__meNameSuccessor;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.meName', T>
): typeof entrypoint_Query__meName;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.nodeField', T>
): typeof entrypoint_Query__nodeField;

export function iso(_isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any>
{
  return function identity<TClientFieldReturn>(
    clientFieldOrEntrypoint: (param: any) => TClientFieldReturn,
  ): (param: any) => TClientFieldReturn {
    return clientFieldOrEntrypoint;
  };
}