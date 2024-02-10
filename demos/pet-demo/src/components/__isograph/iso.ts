import type {IsographEntrypoint} from '@isograph/react';
import entrypoint_Query__HomeRoute from '../__isograph/Query/HomeRoute/entrypoint'
import entrypoint_Query__PetDetailRoute from '../__isograph/Query/PetDetailRoute/entrypoint'
import { Pet__PetBestFriendCard__param } from './Pet/PetBestFriendCard/reader'
import { Pet__PetCheckinsCard__param } from './Pet/PetCheckinsCard/reader'
import { Pet__PetPhraseCard__param } from './Pet/PetPhraseCard/reader'
import { Pet__PetStatsCard__param } from './Pet/PetStatsCard/reader'
import { Pet__PetSummaryCard__param } from './Pet/PetSummaryCard/reader'
import { Pet__PetTaglineCard__param } from './Pet/PetTaglineCard/reader'
import { Pet__PetUpdater__param } from './Pet/PetUpdater/reader'
import { Query__HomeRoute__param } from './Query/HomeRoute/reader'
import { Query__PetDetailRoute__param } from './Query/PetDetailRoute/reader'

type IdentityWithParam<TParam> = <TResolverReturn>(
  x: (param: TParam) => TResolverReturn
) => (param: TParam) => TResolverReturn;

type WhitespaceCharacter = ' ' | '\t' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

type MatchesWhitespaceAndString<
  TString extends string,
  T
> = Whitespace<T> extends `${TString}${string}` ? T : never;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.HomeRoute', T>
): typeof entrypoint_Query__HomeRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetDetailRoute', T>
): typeof entrypoint_Query__PetDetailRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetBestFriendCard', T>
): IdentityWithParam<Pet__PetBestFriendCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetCheckinsCard', T>
): IdentityWithParam<Pet__PetCheckinsCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetPhraseCard', T>
): IdentityWithParam<Pet__PetPhraseCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetStatsCard', T>
): IdentityWithParam<Pet__PetStatsCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetSummaryCard', T>
): IdentityWithParam<Pet__PetSummaryCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetTaglineCard', T>
): IdentityWithParam<Pet__PetTaglineCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetUpdater', T>
): IdentityWithParam<Pet__PetUpdater__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomeRoute', T>
): IdentityWithParam<Query__HomeRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetDetailRoute', T>
): IdentityWithParam<Query__PetDetailRoute__param>;

export function iso(_isographLiteralText: string): IdentityWithParam<any> | IsographEntrypoint<any, any, any>{
  return function identity<TResolverReturn>(
    clientFieldOrEntrypoint: (param: any) => TResolverReturn,
  ): (param: any) => TResolverReturn {
    return clientFieldOrEntrypoint;
  };
}