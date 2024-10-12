import type {IsographEntrypoint} from '@isograph/react';
import { Pokemon__Pokemon__param } from './Pokemon/Pokemon/param_type';
import { Query__HomePage__param } from './Query/HomePage/param_type';
import entrypoint_Query__HomePage from '../__isograph/Query/HomePage/entrypoint';

// This is the type given to regular client fields.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes one parameter
// of type TParam.
type IdentityWithParam<TParam> = <TClientFieldReturn>(
  clientField: (param: TParam) => TClientFieldReturn
) => (param: TParam) => TClientFieldReturn;

// This is the type given it to client fields with @component.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes two parameters.
// The first has type TParam, and the second has type TComponentProps.
//
// TComponentProps becomes the types of the props you must pass
// whenever the @component field is rendered.
type IdentityWithParamComponent<TParam> = <TClientFieldReturn, TComponentProps = Record<string, never>>(
  clientComponentField: (data: TParam, componentProps: TComponentProps) => TClientFieldReturn
) => (data: TParam, componentProps: TComponentProps) => TClientFieldReturn;

type WhitespaceCharacter = ' ' | '\t' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

// This is a recursive TypeScript type that matches strings that
// start with whitespace, followed by TString. So e.g. if we have
// ```
// export function iso<T>(
//   isographLiteralText: T & MatchesWhitespaceAndString<'field Query.foo', T>
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
  param: T & MatchesWhitespaceAndString<'field Pokemon.Pokemon', T>
): IdentityWithParamComponent<Pokemon__Pokemon__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomePage', T>
): IdentityWithParamComponent<Query__HomePage__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.HomePage', T>
): typeof entrypoint_Query__HomePage;

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