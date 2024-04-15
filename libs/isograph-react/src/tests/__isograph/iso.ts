import type {IsographEntrypoint} from '@isograph/react';
import { Query__meNameSuccessor__param } from './Query/meNameSuccessor/param_type';
import { Query__meName__param } from './Query/meName/param_type';
import { Query__nodeField__param } from './Query/nodeField/param_type';
import entrypoint_Query__meNameSuccessor from '../__isograph/Query/meNameSuccessor/entrypoint';
import entrypoint_Query__meName from '../__isograph/Query/meName/entrypoint';
import entrypoint_Query__nodeField from '../__isograph/Query/nodeField/entrypoint';

type IdentityWithParam<TParam> = <TResolverReturn>(
  x: (param: TParam) => TResolverReturn
) => (param: TParam) => TResolverReturn;
type IdentityWithParamComponent<TParam> = <TResolverReturn, TSecondParam = Record<string, never>>(
  x: (data: TParam, secondParam: TSecondParam) => TResolverReturn
) => (data: TParam, secondParam: TSecondParam) => TResolverReturn;

type WhitespaceCharacter = ' ' | '\t' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

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
  return function identity<TResolverReturn>(
    clientFieldOrEntrypoint: (param: any) => TResolverReturn,
  ): (param: any) => TResolverReturn {
    return clientFieldOrEntrypoint;
  };
}