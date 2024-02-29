import type {IsographEntrypoint} from '@isograph/react';
import { Actor__UserLink__param } from './Actor/UserLink/reader'
import { IssueComment__formattedCommentCreationDate__param } from './IssueComment/formattedCommentCreationDate/reader'
import { PullRequest__CommentList__param } from './PullRequest/CommentList/reader'
import { PullRequest__PullRequestLink__param } from './PullRequest/PullRequestLink/reader'
import { PullRequest__createdAtFormatted__param } from './PullRequest/createdAtFormatted/reader'
import { PullRequestConnection__PullRequestTable__param } from './PullRequestConnection/PullRequestTable/reader'
import { Query__Header__param } from './Query/Header/reader'
import { Query__HomePageList__param } from './Query/HomePageList/reader'
import { Query__HomePage__param } from './Query/HomePage/reader'
import { Query__PullRequestDetail__param } from './Query/PullRequestDetail/reader'
import { Query__PullRequest__param } from './Query/PullRequest/reader'
import { Query__RepositoryDetail__param } from './Query/RepositoryDetail/reader'
import { Query__RepositoryPage__param } from './Query/RepositoryPage/reader'
import { Query__UserDetail__param } from './Query/UserDetail/reader'
import { Query__UserPage__param } from './Query/UserPage/reader'
import { Repository__RepositoryLink__param } from './Repository/RepositoryLink/reader'
import { Starrable__IsStarred__param } from './Starrable/IsStarred/reader'
import { User__Avatar__param } from './User/Avatar/reader'
import { User__RepositoryList__param } from './User/RepositoryList/reader'
import entrypoint_Query__HomePage from '../__isograph/Query/HomePage/entrypoint'
import entrypoint_Query__PullRequest from '../__isograph/Query/PullRequest/entrypoint'
import entrypoint_Query__RepositoryPage from '../__isograph/Query/RepositoryPage/entrypoint'
import entrypoint_Query__UserPage from '../__isograph/Query/UserPage/entrypoint'

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
  param: T & MatchesWhitespaceAndString<'field Actor.UserLink', T>
): IdentityWithParamComponent<Actor__UserLink__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field IssueComment.formattedCommentCreationDate', T>
): IdentityWithParam<IssueComment__formattedCommentCreationDate__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequest.CommentList', T>
): IdentityWithParamComponent<PullRequest__CommentList__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequest.PullRequestLink', T>
): IdentityWithParamComponent<PullRequest__PullRequestLink__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequest.createdAtFormatted', T>
): IdentityWithParam<PullRequest__createdAtFormatted__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequestConnection.PullRequestTable', T>
): IdentityWithParamComponent<PullRequestConnection__PullRequestTable__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.Header', T>
): IdentityWithParamComponent<Query__Header__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomePageList', T>
): IdentityWithParamComponent<Query__HomePageList__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomePage', T>
): IdentityWithParamComponent<Query__HomePage__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PullRequestDetail', T>
): IdentityWithParamComponent<Query__PullRequestDetail__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PullRequest', T>
): IdentityWithParamComponent<Query__PullRequest__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.RepositoryDetail', T>
): IdentityWithParamComponent<Query__RepositoryDetail__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.RepositoryPage', T>
): IdentityWithParamComponent<Query__RepositoryPage__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.UserDetail', T>
): IdentityWithParamComponent<Query__UserDetail__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.UserPage', T>
): IdentityWithParamComponent<Query__UserPage__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Repository.RepositoryLink', T>
): IdentityWithParamComponent<Repository__RepositoryLink__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Starrable.IsStarred', T>
): IdentityWithParamComponent<Starrable__IsStarred__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field User.Avatar', T>
): IdentityWithParamComponent<User__Avatar__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field User.RepositoryList', T>
): IdentityWithParamComponent<User__RepositoryList__param>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.HomePage', T>
): typeof entrypoint_Query__HomePage;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.PullRequest', T>
): typeof entrypoint_Query__PullRequest;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.RepositoryPage', T>
): typeof entrypoint_Query__RepositoryPage;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'entrypoint Query.UserPage', T>
): typeof entrypoint_Query__UserPage;

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