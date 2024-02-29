import type {IsographEntrypoint} from '@isograph/react';
import { Pet__FavoritePhraseLoader__param } from './Pet/FavoritePhraseLoader/reader'
import { Pet__PetBestFriendCard__param } from './Pet/PetBestFriendCard/reader'
import { Pet__PetCheckinsCard__param } from './Pet/PetCheckinsCard/reader'
import { Pet__PetPhraseCard__param } from './Pet/PetPhraseCard/reader'
import { Pet__PetStatsCard__param } from './Pet/PetStatsCard/reader'
import { Pet__PetSummaryCard__param } from './Pet/PetSummaryCard/reader'
import { Pet__PetTaglineCard__param } from './Pet/PetTaglineCard/reader'
import { Pet__PetUpdater__param } from './Pet/PetUpdater/reader'
import { Query__HomeRoute__param } from './Query/HomeRoute/reader'
import { Query__PetDetailRoute__param } from './Query/PetDetailRoute/reader'
import { Query__PetFavoritePhrase__param } from './Query/PetFavoritePhrase/reader'
import entrypoint_Query__HomeRoute from '../__isograph/Query/HomeRoute/entrypoint'
import entrypoint_Query__PetDetailRoute from '../__isograph/Query/PetDetailRoute/entrypoint'
import entrypoint_Query__PetFavoritePhrase from '../__isograph/Query/PetFavoritePhrase/entrypoint'

type IdentityWithParam<TParam> = <TResolverReturn>(
  x: (param: TParam) => TResolverReturn
) => (param: TParam) => TResolverReturn;
type IdentityWithParamComponent<TParam> = <TResolverReturn, TSecondParam = {}>(
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
  param: T & MatchesWhitespaceAndString<'field Pet.FavoritePhraseLoader', T>
): IdentityWithParamComponent<Pet__FavoritePhraseLoader__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetBestFriendCard', T>
): IdentityWithParamComponent<Pet__PetBestFriendCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetCheckinsCard', T>
): IdentityWithParamComponent<Pet__PetCheckinsCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetPhraseCard', T>
): IdentityWithParamComponent<Pet__PetPhraseCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetStatsCard', T>
): IdentityWithParamComponent<Pet__PetStatsCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetSummaryCard', T>
): IdentityWithParamComponent<Pet__PetSummaryCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetTaglineCard', T>
): IdentityWithParamComponent<Pet__PetTaglineCard__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Pet.PetUpdater', T>
): IdentityWithParamComponent<Pet__PetUpdater__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomeRoute', T>
): IdentityWithParamComponent<Query__HomeRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetDetailRoute', T>
): IdentityWithParamComponent<Query__PetDetailRoute__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PetFavoritePhrase', T>
): IdentityWithParamComponent<Query__PetFavoritePhrase__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.HomeRoute', T>
): typeof entrypoint_Query__HomeRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetDetailRoute', T>
): typeof entrypoint_Query__PetDetailRoute;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PetFavoritePhrase', T>
): typeof entrypoint_Query__PetFavoritePhrase;

export function iso(_isographLiteralText: string):
  | IdentityWithParam<any>
  | IdentityWithParamComponent<any>
  | IsographEntrypoint<any, any, any>
{
  return function identity<TResolverReturn>(
    clientFieldOrEntrypoint: (param: any) => TResolverReturn,
  ): (param: any) => TResolverReturn {
    return clientFieldOrEntrypoint;
  };
}