import type {IsographEntrypoint} from '@isograph/react';
import entrypoint_Query__HomePage from '../__isograph/Query/HomePage/entrypoint.ts'
import entrypoint_Query__PullRequest from '../__isograph/Query/PullRequest/entrypoint.ts'
import entrypoint_Query__RepositoryPage from '../__isograph/Query/RepositoryPage/entrypoint.ts'
import entrypoint_Query__UserPage from '../__isograph/Query/UserPage/entrypoint.ts'
import { ResolverParameterType as field_Actor__UserLink } from './Actor/UserLink/reader.ts'
import { ResolverParameterType as field_IssueComment__formattedCommentCreationDate } from './IssueComment/formattedCommentCreationDate/reader.ts'
import { ResolverParameterType as field_PullRequest__CommentList } from './PullRequest/CommentList/reader.ts'
import { ResolverParameterType as field_PullRequest__PullRequestLink } from './PullRequest/PullRequestLink/reader.ts'
import { ResolverParameterType as field_PullRequest__createdAtFormatted } from './PullRequest/createdAtFormatted/reader.ts'
import { ResolverParameterType as field_PullRequestConnection__PullRequestTable } from './PullRequestConnection/PullRequestTable/reader.ts'
import { ResolverParameterType as field_Query__Header } from './Query/Header/reader.ts'
import { ResolverParameterType as field_Query__HomePage } from './Query/HomePage/reader.ts'
import { ResolverParameterType as field_Query__HomePageList } from './Query/HomePageList/reader.ts'
import { ResolverParameterType as field_Query__PullRequest } from './Query/PullRequest/reader.ts'
import { ResolverParameterType as field_Query__PullRequestDetail } from './Query/PullRequestDetail/reader.ts'
import { ResolverParameterType as field_Query__RepositoryDetail } from './Query/RepositoryDetail/reader.ts'
import { ResolverParameterType as field_Query__RepositoryPage } from './Query/RepositoryPage/reader.ts'
import { ResolverParameterType as field_Query__UserDetail } from './Query/UserDetail/reader.ts'
import { ResolverParameterType as field_Query__UserPage } from './Query/UserPage/reader.ts'
import { ResolverParameterType as field_Repository__RepositoryLink } from './Repository/RepositoryLink/reader.ts'
import { ResolverParameterType as field_User__Avatar } from './User/Avatar/reader.ts'
import { ResolverParameterType as field_User__RepositoryList } from './User/RepositoryList/reader.ts'

type IdentityWithParam<TParam> = <TResolverReturn>(
  x: (param: TParam) => TResolverReturn
) => (param: TParam) => TResolverReturn;

type WhitespaceCharacter = ' ' | '\n';
type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
  ? Whitespace<In>
  : In;

type MatchesWhitespaceAndString<
  TString extends string,
  T
> = Whitespace<T> extends `${TString}${string}` ? T : never;

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

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Actor.UserLink', T>
): IdentityWithParam<field_Actor__UserLink>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field IssueComment.formattedCommentCreationDate', T>
): IdentityWithParam<field_IssueComment__formattedCommentCreationDate>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequest.CommentList', T>
): IdentityWithParam<field_PullRequest__CommentList>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequest.PullRequestLink', T>
): IdentityWithParam<field_PullRequest__PullRequestLink>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequest.createdAtFormatted', T>
): IdentityWithParam<field_PullRequest__createdAtFormatted>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field PullRequestConnection.PullRequestTable', T>
): IdentityWithParam<field_PullRequestConnection__PullRequestTable>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.Header', T>
): IdentityWithParam<field_Query__Header>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomePage', T>
): IdentityWithParam<field_Query__HomePage>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.HomePageList', T>
): IdentityWithParam<field_Query__HomePageList>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PullRequest', T>
): IdentityWithParam<field_Query__PullRequest>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.PullRequestDetail', T>
): IdentityWithParam<field_Query__PullRequestDetail>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.RepositoryDetail', T>
): IdentityWithParam<field_Query__RepositoryDetail>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.RepositoryPage', T>
): IdentityWithParam<field_Query__RepositoryPage>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.UserDetail', T>
): IdentityWithParam<field_Query__UserDetail>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Query.UserPage', T>
): IdentityWithParam<field_Query__UserPage>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field Repository.RepositoryLink', T>
): IdentityWithParam<field_Repository__RepositoryLink>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field User.Avatar', T>
): IdentityWithParam<field_User__Avatar>;

export function iso<T>(
  param: T & MatchesWhitespaceAndString<'field User.RepositoryList', T>
): IdentityWithParam<field_User__RepositoryList>;

export function iso(_queryText: string): IdentityWithParam<any> | IsographEntrypoint<any, any, any>{
  return function identity<TResolverReturn>(
    x: (param: any) => TResolverReturn,
  ): (param: any) => TResolverReturn {
    return x;
  };
}