import type { IsographEntrypoint, ResolverFirstParameter, Variables } from '@isograph/react';
import { type Query__meNameSuccessor__param } from './Query/meNameSuccessor/param_type';
import { type Query__meName__param } from './Query/meName/param_type';
import { type Query__nodeField__param } from './Query/nodeField/param_type';
import entrypoint_Query__meNameSuccessor from '../__isograph/Query/meNameSuccessor/entrypoint';
import entrypoint_Query__meName from '../__isograph/Query/meName/entrypoint';
import entrypoint_Query__nodeField from '../__isograph/Query/nodeField/entrypoint';

// This is the type given to regular client fields.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes one parameter
// of type TParam.
type IdentityWithParam<TParam extends object> = <TClientFieldReturn, TVariables = Variables>(
  clientField: (param: TParam) => TClientFieldReturn
) => (param: ResolverFirstParameter<TParam, TVariables>) => TClientFieldReturn;

// This is the type given it to client fields with @component.
// This means that the type of the exported iso literal is exactly
// the type of the passed-in function, which takes two parameters.
// The first has type TParam, and the second has type TComponentProps.
//
// TComponentProps becomes the types of the props you must pass
// whenever the @component field is rendered.
type IdentityWithParamComponent<TParam extends object> = <
  TClientFieldReturn,
  TComponentProps = Record<string, never>,
  TVariables = Variables
>(
  clientComponentField: (data: TParam, componentProps: TComponentProps) => TClientFieldReturn
) => (data: ResolverFirstParameter<TParam, TVariables>, componentProps: TComponentProps) => TClientFieldReturn;

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
  throw new Error('iso: Unexpected invocation at runtime. Either the Babel transform ' +
      'was not set up, or it failed to identify this call site. Make sure it ' +
      'is being used verbatim as `iso`.');
}