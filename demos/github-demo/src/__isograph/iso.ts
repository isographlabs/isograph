type IdentityWithParam<TParam> = <TResolverReturn>(
            x: (param: TParam) => TResolverReturn
        ) => (param: TParam) => TResolverReturn;

      type WhitespaceCharacter = ' ' | '
';
      type Whitespace<In> = In extends `${WhitespaceCharacter}${infer In}`
        ? Whitespace<In>
        : In;
      
      type MatchesWhitespaceAndString<
        TString extends string,
        T
      > = Whitespace<T> extends `${TString}${string}` ? T : never;
    import Query__UserPage from '../__isograph/Query/UserPage/entrypoint.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'entrypoint Query.UserPage', T>
        ): IdentityWithParam<field_Query__UserPage>;

import Query__RepositoryPage from '../__isograph/Query/RepositoryPage/entrypoint.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'entrypoint Query.RepositoryPage', T>
        ): IdentityWithParam<field_Query__RepositoryPage>;

import Query__HomePage from '../__isograph/Query/HomePage/entrypoint.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'entrypoint Query.HomePage', T>
        ): IdentityWithParam<field_Query__HomePage>;

import Query__PullRequest from '../__isograph/Query/PullRequest/entrypoint.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'entrypoint Query.PullRequest', T>
        ): IdentityWithParam<field_Query__PullRequest>;
import { ResolverParameterType as field_AddedToMergeQueueEvent____refetch } from './AddedToMergeQueueEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field AddedToMergeQueueEvent.__refetch', T>
        ): IdentityWithParam<field_AddedToMergeQueueEvent____refetch>;

import { ResolverParameterType as field_AddedToProjectEvent____refetch } from './AddedToProjectEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field AddedToProjectEvent.__refetch', T>
        ): IdentityWithParam<field_AddedToProjectEvent____refetch>;

import { ResolverParameterType as field_App____refetch } from './App/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field App.__refetch', T>
        ): IdentityWithParam<field_App____refetch>;

import { ResolverParameterType as field_AssignedEvent____refetch } from './AssignedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field AssignedEvent.__refetch', T>
        ): IdentityWithParam<field_AssignedEvent____refetch>;

import { ResolverParameterType as field_AutoMergeDisabledEvent____refetch } from './AutoMergeDisabledEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field AutoMergeDisabledEvent.__refetch', T>
        ): IdentityWithParam<field_AutoMergeDisabledEvent____refetch>;

import { ResolverParameterType as field_AutoMergeEnabledEvent____refetch } from './AutoMergeEnabledEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field AutoMergeEnabledEvent.__refetch', T>
        ): IdentityWithParam<field_AutoMergeEnabledEvent____refetch>;

import { ResolverParameterType as field_AutoRebaseEnabledEvent____refetch } from './AutoRebaseEnabledEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field AutoRebaseEnabledEvent.__refetch', T>
        ): IdentityWithParam<field_AutoRebaseEnabledEvent____refetch>;

import { ResolverParameterType as field_AutoSquashEnabledEvent____refetch } from './AutoSquashEnabledEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field AutoSquashEnabledEvent.__refetch', T>
        ): IdentityWithParam<field_AutoSquashEnabledEvent____refetch>;

import { ResolverParameterType as field_AutomaticBaseChangeFailedEvent____refetch } from './AutomaticBaseChangeFailedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field AutomaticBaseChangeFailedEvent.__refetch', T>
        ): IdentityWithParam<field_AutomaticBaseChangeFailedEvent____refetch>;

import { ResolverParameterType as field_AutomaticBaseChangeSucceededEvent____refetch } from './AutomaticBaseChangeSucceededEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field AutomaticBaseChangeSucceededEvent.__refetch', T>
        ): IdentityWithParam<field_AutomaticBaseChangeSucceededEvent____refetch>;

import { ResolverParameterType as field_BaseRefChangedEvent____refetch } from './BaseRefChangedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field BaseRefChangedEvent.__refetch', T>
        ): IdentityWithParam<field_BaseRefChangedEvent____refetch>;

import { ResolverParameterType as field_BaseRefDeletedEvent____refetch } from './BaseRefDeletedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field BaseRefDeletedEvent.__refetch', T>
        ): IdentityWithParam<field_BaseRefDeletedEvent____refetch>;

import { ResolverParameterType as field_BaseRefForcePushedEvent____refetch } from './BaseRefForcePushedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field BaseRefForcePushedEvent.__refetch', T>
        ): IdentityWithParam<field_BaseRefForcePushedEvent____refetch>;

import { ResolverParameterType as field_Blob____refetch } from './Blob/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Blob.__refetch', T>
        ): IdentityWithParam<field_Blob____refetch>;

import { ResolverParameterType as field_Bot____refetch } from './Bot/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Bot.__refetch', T>
        ): IdentityWithParam<field_Bot____refetch>;

import { ResolverParameterType as field_BranchProtectionRule____refetch } from './BranchProtectionRule/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field BranchProtectionRule.__refetch', T>
        ): IdentityWithParam<field_BranchProtectionRule____refetch>;

import { ResolverParameterType as field_BypassForcePushAllowance____refetch } from './BypassForcePushAllowance/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field BypassForcePushAllowance.__refetch', T>
        ): IdentityWithParam<field_BypassForcePushAllowance____refetch>;

import { ResolverParameterType as field_BypassPullRequestAllowance____refetch } from './BypassPullRequestAllowance/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field BypassPullRequestAllowance.__refetch', T>
        ): IdentityWithParam<field_BypassPullRequestAllowance____refetch>;

import { ResolverParameterType as field_CWE____refetch } from './CWE/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field CWE.__refetch', T>
        ): IdentityWithParam<field_CWE____refetch>;

import { ResolverParameterType as field_CheckRun____refetch } from './CheckRun/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field CheckRun.__refetch', T>
        ): IdentityWithParam<field_CheckRun____refetch>;

import { ResolverParameterType as field_CheckSuite____refetch } from './CheckSuite/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field CheckSuite.__refetch', T>
        ): IdentityWithParam<field_CheckSuite____refetch>;

import { ResolverParameterType as field_ClosedEvent____refetch } from './ClosedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ClosedEvent.__refetch', T>
        ): IdentityWithParam<field_ClosedEvent____refetch>;

import { ResolverParameterType as field_CodeOfConduct____refetch } from './CodeOfConduct/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field CodeOfConduct.__refetch', T>
        ): IdentityWithParam<field_CodeOfConduct____refetch>;

import { ResolverParameterType as field_Comment____refetch } from './Comment/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Comment.__refetch', T>
        ): IdentityWithParam<field_Comment____refetch>;

import { ResolverParameterType as field_CommentDeletedEvent____refetch } from './CommentDeletedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field CommentDeletedEvent.__refetch', T>
        ): IdentityWithParam<field_CommentDeletedEvent____refetch>;

import { ResolverParameterType as field_Commit____refetch } from './Commit/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Commit.__refetch', T>
        ): IdentityWithParam<field_Commit____refetch>;

import { ResolverParameterType as field_CommitComment____refetch } from './CommitComment/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field CommitComment.__refetch', T>
        ): IdentityWithParam<field_CommitComment____refetch>;

import { ResolverParameterType as field_CommitCommentThread____refetch } from './CommitCommentThread/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field CommitCommentThread.__refetch', T>
        ): IdentityWithParam<field_CommitCommentThread____refetch>;

import { ResolverParameterType as field_Comparison____refetch } from './Comparison/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Comparison.__refetch', T>
        ): IdentityWithParam<field_Comparison____refetch>;

import { ResolverParameterType as field_ConnectedEvent____refetch } from './ConnectedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ConnectedEvent.__refetch', T>
        ): IdentityWithParam<field_ConnectedEvent____refetch>;

import { ResolverParameterType as field_ConvertToDraftEvent____refetch } from './ConvertToDraftEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ConvertToDraftEvent.__refetch', T>
        ): IdentityWithParam<field_ConvertToDraftEvent____refetch>;

import { ResolverParameterType as field_ConvertedNoteToIssueEvent____refetch } from './ConvertedNoteToIssueEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ConvertedNoteToIssueEvent.__refetch', T>
        ): IdentityWithParam<field_ConvertedNoteToIssueEvent____refetch>;

import { ResolverParameterType as field_ConvertedToDiscussionEvent____refetch } from './ConvertedToDiscussionEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ConvertedToDiscussionEvent.__refetch', T>
        ): IdentityWithParam<field_ConvertedToDiscussionEvent____refetch>;

import { ResolverParameterType as field_CrossReferencedEvent____refetch } from './CrossReferencedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field CrossReferencedEvent.__refetch', T>
        ): IdentityWithParam<field_CrossReferencedEvent____refetch>;

import { ResolverParameterType as field_DemilestonedEvent____refetch } from './DemilestonedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DemilestonedEvent.__refetch', T>
        ): IdentityWithParam<field_DemilestonedEvent____refetch>;

import { ResolverParameterType as field_DependencyGraphManifest____refetch } from './DependencyGraphManifest/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DependencyGraphManifest.__refetch', T>
        ): IdentityWithParam<field_DependencyGraphManifest____refetch>;

import { ResolverParameterType as field_DeployKey____refetch } from './DeployKey/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DeployKey.__refetch', T>
        ): IdentityWithParam<field_DeployKey____refetch>;

import { ResolverParameterType as field_DeployedEvent____refetch } from './DeployedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DeployedEvent.__refetch', T>
        ): IdentityWithParam<field_DeployedEvent____refetch>;

import { ResolverParameterType as field_Deployment____refetch } from './Deployment/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Deployment.__refetch', T>
        ): IdentityWithParam<field_Deployment____refetch>;

import { ResolverParameterType as field_DeploymentEnvironmentChangedEvent____refetch } from './DeploymentEnvironmentChangedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DeploymentEnvironmentChangedEvent.__refetch', T>
        ): IdentityWithParam<field_DeploymentEnvironmentChangedEvent____refetch>;

import { ResolverParameterType as field_DeploymentReview____refetch } from './DeploymentReview/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DeploymentReview.__refetch', T>
        ): IdentityWithParam<field_DeploymentReview____refetch>;

import { ResolverParameterType as field_DeploymentStatus____refetch } from './DeploymentStatus/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DeploymentStatus.__refetch', T>
        ): IdentityWithParam<field_DeploymentStatus____refetch>;

import { ResolverParameterType as field_DisconnectedEvent____refetch } from './DisconnectedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DisconnectedEvent.__refetch', T>
        ): IdentityWithParam<field_DisconnectedEvent____refetch>;

import { ResolverParameterType as field_Discussion____refetch } from './Discussion/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Discussion.__refetch', T>
        ): IdentityWithParam<field_Discussion____refetch>;

import { ResolverParameterType as field_DiscussionCategory____refetch } from './DiscussionCategory/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DiscussionCategory.__refetch', T>
        ): IdentityWithParam<field_DiscussionCategory____refetch>;

import { ResolverParameterType as field_DiscussionComment____refetch } from './DiscussionComment/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DiscussionComment.__refetch', T>
        ): IdentityWithParam<field_DiscussionComment____refetch>;

import { ResolverParameterType as field_DiscussionPoll____refetch } from './DiscussionPoll/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DiscussionPoll.__refetch', T>
        ): IdentityWithParam<field_DiscussionPoll____refetch>;

import { ResolverParameterType as field_DiscussionPollOption____refetch } from './DiscussionPollOption/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DiscussionPollOption.__refetch', T>
        ): IdentityWithParam<field_DiscussionPollOption____refetch>;

import { ResolverParameterType as field_DraftIssue____refetch } from './DraftIssue/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field DraftIssue.__refetch', T>
        ): IdentityWithParam<field_DraftIssue____refetch>;

import { ResolverParameterType as field_Enterprise____refetch } from './Enterprise/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Enterprise.__refetch', T>
        ): IdentityWithParam<field_Enterprise____refetch>;

import { ResolverParameterType as field_EnterpriseAdministratorInvitation____refetch } from './EnterpriseAdministratorInvitation/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field EnterpriseAdministratorInvitation.__refetch', T>
        ): IdentityWithParam<field_EnterpriseAdministratorInvitation____refetch>;

import { ResolverParameterType as field_EnterpriseIdentityProvider____refetch } from './EnterpriseIdentityProvider/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field EnterpriseIdentityProvider.__refetch', T>
        ): IdentityWithParam<field_EnterpriseIdentityProvider____refetch>;

import { ResolverParameterType as field_EnterpriseRepositoryInfo____refetch } from './EnterpriseRepositoryInfo/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field EnterpriseRepositoryInfo.__refetch', T>
        ): IdentityWithParam<field_EnterpriseRepositoryInfo____refetch>;

import { ResolverParameterType as field_EnterpriseServerInstallation____refetch } from './EnterpriseServerInstallation/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field EnterpriseServerInstallation.__refetch', T>
        ): IdentityWithParam<field_EnterpriseServerInstallation____refetch>;

import { ResolverParameterType as field_EnterpriseServerUserAccount____refetch } from './EnterpriseServerUserAccount/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field EnterpriseServerUserAccount.__refetch', T>
        ): IdentityWithParam<field_EnterpriseServerUserAccount____refetch>;

import { ResolverParameterType as field_EnterpriseServerUserAccountEmail____refetch } from './EnterpriseServerUserAccountEmail/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field EnterpriseServerUserAccountEmail.__refetch', T>
        ): IdentityWithParam<field_EnterpriseServerUserAccountEmail____refetch>;

import { ResolverParameterType as field_EnterpriseServerUserAccountsUpload____refetch } from './EnterpriseServerUserAccountsUpload/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field EnterpriseServerUserAccountsUpload.__refetch', T>
        ): IdentityWithParam<field_EnterpriseServerUserAccountsUpload____refetch>;

import { ResolverParameterType as field_EnterpriseUserAccount____refetch } from './EnterpriseUserAccount/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field EnterpriseUserAccount.__refetch', T>
        ): IdentityWithParam<field_EnterpriseUserAccount____refetch>;

import { ResolverParameterType as field_Environment____refetch } from './Environment/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Environment.__refetch', T>
        ): IdentityWithParam<field_Environment____refetch>;

import { ResolverParameterType as field_ExternalIdentity____refetch } from './ExternalIdentity/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ExternalIdentity.__refetch', T>
        ): IdentityWithParam<field_ExternalIdentity____refetch>;

import { ResolverParameterType as field_Gist____refetch } from './Gist/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Gist.__refetch', T>
        ): IdentityWithParam<field_Gist____refetch>;

import { ResolverParameterType as field_GistComment____refetch } from './GistComment/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field GistComment.__refetch', T>
        ): IdentityWithParam<field_GistComment____refetch>;

import { ResolverParameterType as field_GitObject____refetch } from './GitObject/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field GitObject.__refetch', T>
        ): IdentityWithParam<field_GitObject____refetch>;

import { ResolverParameterType as field_HeadRefDeletedEvent____refetch } from './HeadRefDeletedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field HeadRefDeletedEvent.__refetch', T>
        ): IdentityWithParam<field_HeadRefDeletedEvent____refetch>;

import { ResolverParameterType as field_HeadRefForcePushedEvent____refetch } from './HeadRefForcePushedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field HeadRefForcePushedEvent.__refetch', T>
        ): IdentityWithParam<field_HeadRefForcePushedEvent____refetch>;

import { ResolverParameterType as field_HeadRefRestoredEvent____refetch } from './HeadRefRestoredEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field HeadRefRestoredEvent.__refetch', T>
        ): IdentityWithParam<field_HeadRefRestoredEvent____refetch>;

import { ResolverParameterType as field_IpAllowListEntry____refetch } from './IpAllowListEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field IpAllowListEntry.__refetch', T>
        ): IdentityWithParam<field_IpAllowListEntry____refetch>;

import { ResolverParameterType as field_Issue____refetch } from './Issue/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Issue.__refetch', T>
        ): IdentityWithParam<field_Issue____refetch>;

import { ResolverParameterType as field_IssueComment____refetch } from './IssueComment/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field IssueComment.__refetch', T>
        ): IdentityWithParam<field_IssueComment____refetch>;

import { ResolverParameterType as field_Label____refetch } from './Label/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Label.__refetch', T>
        ): IdentityWithParam<field_Label____refetch>;

import { ResolverParameterType as field_LabeledEvent____refetch } from './LabeledEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field LabeledEvent.__refetch', T>
        ): IdentityWithParam<field_LabeledEvent____refetch>;

import { ResolverParameterType as field_Language____refetch } from './Language/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Language.__refetch', T>
        ): IdentityWithParam<field_Language____refetch>;

import { ResolverParameterType as field_License____refetch } from './License/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field License.__refetch', T>
        ): IdentityWithParam<field_License____refetch>;

import { ResolverParameterType as field_LinkedBranch____refetch } from './LinkedBranch/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field LinkedBranch.__refetch', T>
        ): IdentityWithParam<field_LinkedBranch____refetch>;

import { ResolverParameterType as field_LockedEvent____refetch } from './LockedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field LockedEvent.__refetch', T>
        ): IdentityWithParam<field_LockedEvent____refetch>;

import { ResolverParameterType as field_Mannequin____refetch } from './Mannequin/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Mannequin.__refetch', T>
        ): IdentityWithParam<field_Mannequin____refetch>;

import { ResolverParameterType as field_MarkedAsDuplicateEvent____refetch } from './MarkedAsDuplicateEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MarkedAsDuplicateEvent.__refetch', T>
        ): IdentityWithParam<field_MarkedAsDuplicateEvent____refetch>;

import { ResolverParameterType as field_MarketplaceCategory____refetch } from './MarketplaceCategory/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MarketplaceCategory.__refetch', T>
        ): IdentityWithParam<field_MarketplaceCategory____refetch>;

import { ResolverParameterType as field_MarketplaceListing____refetch } from './MarketplaceListing/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MarketplaceListing.__refetch', T>
        ): IdentityWithParam<field_MarketplaceListing____refetch>;

import { ResolverParameterType as field_MembersCanDeleteReposClearAuditEntry____refetch } from './MembersCanDeleteReposClearAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MembersCanDeleteReposClearAuditEntry.__refetch', T>
        ): IdentityWithParam<field_MembersCanDeleteReposClearAuditEntry____refetch>;

import { ResolverParameterType as field_MembersCanDeleteReposDisableAuditEntry____refetch } from './MembersCanDeleteReposDisableAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MembersCanDeleteReposDisableAuditEntry.__refetch', T>
        ): IdentityWithParam<field_MembersCanDeleteReposDisableAuditEntry____refetch>;

import { ResolverParameterType as field_MembersCanDeleteReposEnableAuditEntry____refetch } from './MembersCanDeleteReposEnableAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MembersCanDeleteReposEnableAuditEntry.__refetch', T>
        ): IdentityWithParam<field_MembersCanDeleteReposEnableAuditEntry____refetch>;

import { ResolverParameterType as field_MentionedEvent____refetch } from './MentionedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MentionedEvent.__refetch', T>
        ): IdentityWithParam<field_MentionedEvent____refetch>;

import { ResolverParameterType as field_MergeQueue____refetch } from './MergeQueue/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MergeQueue.__refetch', T>
        ): IdentityWithParam<field_MergeQueue____refetch>;

import { ResolverParameterType as field_MergeQueueEntry____refetch } from './MergeQueueEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MergeQueueEntry.__refetch', T>
        ): IdentityWithParam<field_MergeQueueEntry____refetch>;

import { ResolverParameterType as field_MergedEvent____refetch } from './MergedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MergedEvent.__refetch', T>
        ): IdentityWithParam<field_MergedEvent____refetch>;

import { ResolverParameterType as field_Migration____refetch } from './Migration/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Migration.__refetch', T>
        ): IdentityWithParam<field_Migration____refetch>;

import { ResolverParameterType as field_MigrationSource____refetch } from './MigrationSource/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MigrationSource.__refetch', T>
        ): IdentityWithParam<field_MigrationSource____refetch>;

import { ResolverParameterType as field_Milestone____refetch } from './Milestone/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Milestone.__refetch', T>
        ): IdentityWithParam<field_Milestone____refetch>;

import { ResolverParameterType as field_MilestonedEvent____refetch } from './MilestonedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MilestonedEvent.__refetch', T>
        ): IdentityWithParam<field_MilestonedEvent____refetch>;

import { ResolverParameterType as field_MovedColumnsInProjectEvent____refetch } from './MovedColumnsInProjectEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field MovedColumnsInProjectEvent.__refetch', T>
        ): IdentityWithParam<field_MovedColumnsInProjectEvent____refetch>;

import { ResolverParameterType as field_Node____refetch } from './Node/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Node.__refetch', T>
        ): IdentityWithParam<field_Node____refetch>;

import { ResolverParameterType as field_OIDCProvider____refetch } from './OIDCProvider/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OIDCProvider.__refetch', T>
        ): IdentityWithParam<field_OIDCProvider____refetch>;

import { ResolverParameterType as field_OauthApplicationCreateAuditEntry____refetch } from './OauthApplicationCreateAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OauthApplicationCreateAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OauthApplicationCreateAuditEntry____refetch>;

import { ResolverParameterType as field_OrgAddBillingManagerAuditEntry____refetch } from './OrgAddBillingManagerAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgAddBillingManagerAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgAddBillingManagerAuditEntry____refetch>;

import { ResolverParameterType as field_OrgAddMemberAuditEntry____refetch } from './OrgAddMemberAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgAddMemberAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgAddMemberAuditEntry____refetch>;

import { ResolverParameterType as field_OrgBlockUserAuditEntry____refetch } from './OrgBlockUserAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgBlockUserAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgBlockUserAuditEntry____refetch>;

import { ResolverParameterType as field_OrgConfigDisableCollaboratorsOnlyAuditEntry____refetch } from './OrgConfigDisableCollaboratorsOnlyAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgConfigDisableCollaboratorsOnlyAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgConfigDisableCollaboratorsOnlyAuditEntry____refetch>;

import { ResolverParameterType as field_OrgConfigEnableCollaboratorsOnlyAuditEntry____refetch } from './OrgConfigEnableCollaboratorsOnlyAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgConfigEnableCollaboratorsOnlyAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgConfigEnableCollaboratorsOnlyAuditEntry____refetch>;

import { ResolverParameterType as field_OrgCreateAuditEntry____refetch } from './OrgCreateAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgCreateAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgCreateAuditEntry____refetch>;

import { ResolverParameterType as field_OrgDisableOauthAppRestrictionsAuditEntry____refetch } from './OrgDisableOauthAppRestrictionsAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgDisableOauthAppRestrictionsAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgDisableOauthAppRestrictionsAuditEntry____refetch>;

import { ResolverParameterType as field_OrgDisableSamlAuditEntry____refetch } from './OrgDisableSamlAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgDisableSamlAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgDisableSamlAuditEntry____refetch>;

import { ResolverParameterType as field_OrgDisableTwoFactorRequirementAuditEntry____refetch } from './OrgDisableTwoFactorRequirementAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgDisableTwoFactorRequirementAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgDisableTwoFactorRequirementAuditEntry____refetch>;

import { ResolverParameterType as field_OrgEnableOauthAppRestrictionsAuditEntry____refetch } from './OrgEnableOauthAppRestrictionsAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgEnableOauthAppRestrictionsAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgEnableOauthAppRestrictionsAuditEntry____refetch>;

import { ResolverParameterType as field_OrgEnableSamlAuditEntry____refetch } from './OrgEnableSamlAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgEnableSamlAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgEnableSamlAuditEntry____refetch>;

import { ResolverParameterType as field_OrgEnableTwoFactorRequirementAuditEntry____refetch } from './OrgEnableTwoFactorRequirementAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgEnableTwoFactorRequirementAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgEnableTwoFactorRequirementAuditEntry____refetch>;

import { ResolverParameterType as field_OrgInviteMemberAuditEntry____refetch } from './OrgInviteMemberAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgInviteMemberAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgInviteMemberAuditEntry____refetch>;

import { ResolverParameterType as field_OrgInviteToBusinessAuditEntry____refetch } from './OrgInviteToBusinessAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgInviteToBusinessAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgInviteToBusinessAuditEntry____refetch>;

import { ResolverParameterType as field_OrgOauthAppAccessApprovedAuditEntry____refetch } from './OrgOauthAppAccessApprovedAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgOauthAppAccessApprovedAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgOauthAppAccessApprovedAuditEntry____refetch>;

import { ResolverParameterType as field_OrgOauthAppAccessDeniedAuditEntry____refetch } from './OrgOauthAppAccessDeniedAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgOauthAppAccessDeniedAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgOauthAppAccessDeniedAuditEntry____refetch>;

import { ResolverParameterType as field_OrgOauthAppAccessRequestedAuditEntry____refetch } from './OrgOauthAppAccessRequestedAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgOauthAppAccessRequestedAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgOauthAppAccessRequestedAuditEntry____refetch>;

import { ResolverParameterType as field_OrgRemoveBillingManagerAuditEntry____refetch } from './OrgRemoveBillingManagerAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgRemoveBillingManagerAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgRemoveBillingManagerAuditEntry____refetch>;

import { ResolverParameterType as field_OrgRemoveMemberAuditEntry____refetch } from './OrgRemoveMemberAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgRemoveMemberAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgRemoveMemberAuditEntry____refetch>;

import { ResolverParameterType as field_OrgRemoveOutsideCollaboratorAuditEntry____refetch } from './OrgRemoveOutsideCollaboratorAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgRemoveOutsideCollaboratorAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgRemoveOutsideCollaboratorAuditEntry____refetch>;

import { ResolverParameterType as field_OrgRestoreMemberAuditEntry____refetch } from './OrgRestoreMemberAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgRestoreMemberAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgRestoreMemberAuditEntry____refetch>;

import { ResolverParameterType as field_OrgUnblockUserAuditEntry____refetch } from './OrgUnblockUserAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgUnblockUserAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgUnblockUserAuditEntry____refetch>;

import { ResolverParameterType as field_OrgUpdateDefaultRepositoryPermissionAuditEntry____refetch } from './OrgUpdateDefaultRepositoryPermissionAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgUpdateDefaultRepositoryPermissionAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgUpdateDefaultRepositoryPermissionAuditEntry____refetch>;

import { ResolverParameterType as field_OrgUpdateMemberAuditEntry____refetch } from './OrgUpdateMemberAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgUpdateMemberAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgUpdateMemberAuditEntry____refetch>;

import { ResolverParameterType as field_OrgUpdateMemberRepositoryCreationPermissionAuditEntry____refetch } from './OrgUpdateMemberRepositoryCreationPermissionAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgUpdateMemberRepositoryCreationPermissionAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgUpdateMemberRepositoryCreationPermissionAuditEntry____refetch>;

import { ResolverParameterType as field_OrgUpdateMemberRepositoryInvitationPermissionAuditEntry____refetch } from './OrgUpdateMemberRepositoryInvitationPermissionAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrgUpdateMemberRepositoryInvitationPermissionAuditEntry.__refetch', T>
        ): IdentityWithParam<field_OrgUpdateMemberRepositoryInvitationPermissionAuditEntry____refetch>;

import { ResolverParameterType as field_Organization____refetch } from './Organization/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Organization.__refetch', T>
        ): IdentityWithParam<field_Organization____refetch>;

import { ResolverParameterType as field_OrganizationIdentityProvider____refetch } from './OrganizationIdentityProvider/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrganizationIdentityProvider.__refetch', T>
        ): IdentityWithParam<field_OrganizationIdentityProvider____refetch>;

import { ResolverParameterType as field_OrganizationInvitation____refetch } from './OrganizationInvitation/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrganizationInvitation.__refetch', T>
        ): IdentityWithParam<field_OrganizationInvitation____refetch>;

import { ResolverParameterType as field_OrganizationMigration____refetch } from './OrganizationMigration/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field OrganizationMigration.__refetch', T>
        ): IdentityWithParam<field_OrganizationMigration____refetch>;

import { ResolverParameterType as field_Package____refetch } from './Package/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Package.__refetch', T>
        ): IdentityWithParam<field_Package____refetch>;

import { ResolverParameterType as field_PackageFile____refetch } from './PackageFile/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PackageFile.__refetch', T>
        ): IdentityWithParam<field_PackageFile____refetch>;

import { ResolverParameterType as field_PackageOwner____refetch } from './PackageOwner/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PackageOwner.__refetch', T>
        ): IdentityWithParam<field_PackageOwner____refetch>;

import { ResolverParameterType as field_PackageTag____refetch } from './PackageTag/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PackageTag.__refetch', T>
        ): IdentityWithParam<field_PackageTag____refetch>;

import { ResolverParameterType as field_PackageVersion____refetch } from './PackageVersion/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PackageVersion.__refetch', T>
        ): IdentityWithParam<field_PackageVersion____refetch>;

import { ResolverParameterType as field_PinnedDiscussion____refetch } from './PinnedDiscussion/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PinnedDiscussion.__refetch', T>
        ): IdentityWithParam<field_PinnedDiscussion____refetch>;

import { ResolverParameterType as field_PinnedEvent____refetch } from './PinnedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PinnedEvent.__refetch', T>
        ): IdentityWithParam<field_PinnedEvent____refetch>;

import { ResolverParameterType as field_PinnedIssue____refetch } from './PinnedIssue/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PinnedIssue.__refetch', T>
        ): IdentityWithParam<field_PinnedIssue____refetch>;

import { ResolverParameterType as field_PrivateRepositoryForkingDisableAuditEntry____refetch } from './PrivateRepositoryForkingDisableAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PrivateRepositoryForkingDisableAuditEntry.__refetch', T>
        ): IdentityWithParam<field_PrivateRepositoryForkingDisableAuditEntry____refetch>;

import { ResolverParameterType as field_PrivateRepositoryForkingEnableAuditEntry____refetch } from './PrivateRepositoryForkingEnableAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PrivateRepositoryForkingEnableAuditEntry.__refetch', T>
        ): IdentityWithParam<field_PrivateRepositoryForkingEnableAuditEntry____refetch>;

import { ResolverParameterType as field_ProfileOwner____refetch } from './ProfileOwner/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProfileOwner.__refetch', T>
        ): IdentityWithParam<field_ProfileOwner____refetch>;

import { ResolverParameterType as field_Project____refetch } from './Project/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Project.__refetch', T>
        ): IdentityWithParam<field_Project____refetch>;

import { ResolverParameterType as field_ProjectCard____refetch } from './ProjectCard/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectCard.__refetch', T>
        ): IdentityWithParam<field_ProjectCard____refetch>;

import { ResolverParameterType as field_ProjectColumn____refetch } from './ProjectColumn/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectColumn.__refetch', T>
        ): IdentityWithParam<field_ProjectColumn____refetch>;

import { ResolverParameterType as field_ProjectOwner____refetch } from './ProjectOwner/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectOwner.__refetch', T>
        ): IdentityWithParam<field_ProjectOwner____refetch>;

import { ResolverParameterType as field_ProjectV2____refetch } from './ProjectV2/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2.__refetch', T>
        ): IdentityWithParam<field_ProjectV2____refetch>;

import { ResolverParameterType as field_ProjectV2Field____refetch } from './ProjectV2Field/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2Field.__refetch', T>
        ): IdentityWithParam<field_ProjectV2Field____refetch>;

import { ResolverParameterType as field_ProjectV2FieldCommon____refetch } from './ProjectV2FieldCommon/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2FieldCommon.__refetch', T>
        ): IdentityWithParam<field_ProjectV2FieldCommon____refetch>;

import { ResolverParameterType as field_ProjectV2Item____refetch } from './ProjectV2Item/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2Item.__refetch', T>
        ): IdentityWithParam<field_ProjectV2Item____refetch>;

import { ResolverParameterType as field_ProjectV2ItemFieldDateValue____refetch } from './ProjectV2ItemFieldDateValue/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2ItemFieldDateValue.__refetch', T>
        ): IdentityWithParam<field_ProjectV2ItemFieldDateValue____refetch>;

import { ResolverParameterType as field_ProjectV2ItemFieldIterationValue____refetch } from './ProjectV2ItemFieldIterationValue/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2ItemFieldIterationValue.__refetch', T>
        ): IdentityWithParam<field_ProjectV2ItemFieldIterationValue____refetch>;

import { ResolverParameterType as field_ProjectV2ItemFieldNumberValue____refetch } from './ProjectV2ItemFieldNumberValue/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2ItemFieldNumberValue.__refetch', T>
        ): IdentityWithParam<field_ProjectV2ItemFieldNumberValue____refetch>;

import { ResolverParameterType as field_ProjectV2ItemFieldSingleSelectValue____refetch } from './ProjectV2ItemFieldSingleSelectValue/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2ItemFieldSingleSelectValue.__refetch', T>
        ): IdentityWithParam<field_ProjectV2ItemFieldSingleSelectValue____refetch>;

import { ResolverParameterType as field_ProjectV2ItemFieldTextValue____refetch } from './ProjectV2ItemFieldTextValue/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2ItemFieldTextValue.__refetch', T>
        ): IdentityWithParam<field_ProjectV2ItemFieldTextValue____refetch>;

import { ResolverParameterType as field_ProjectV2ItemFieldValueCommon____refetch } from './ProjectV2ItemFieldValueCommon/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2ItemFieldValueCommon.__refetch', T>
        ): IdentityWithParam<field_ProjectV2ItemFieldValueCommon____refetch>;

import { ResolverParameterType as field_ProjectV2IterationField____refetch } from './ProjectV2IterationField/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2IterationField.__refetch', T>
        ): IdentityWithParam<field_ProjectV2IterationField____refetch>;

import { ResolverParameterType as field_ProjectV2IterationFieldIteration____refetch } from './ProjectV2IterationFieldIteration/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2IterationFieldIteration.__refetch', T>
        ): IdentityWithParam<field_ProjectV2IterationFieldIteration____refetch>;

import { ResolverParameterType as field_ProjectV2Owner____refetch } from './ProjectV2Owner/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2Owner.__refetch', T>
        ): IdentityWithParam<field_ProjectV2Owner____refetch>;

import { ResolverParameterType as field_ProjectV2SingleSelectField____refetch } from './ProjectV2SingleSelectField/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2SingleSelectField.__refetch', T>
        ): IdentityWithParam<field_ProjectV2SingleSelectField____refetch>;

import { ResolverParameterType as field_ProjectV2SingleSelectFieldOption____refetch } from './ProjectV2SingleSelectFieldOption/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2SingleSelectFieldOption.__refetch', T>
        ): IdentityWithParam<field_ProjectV2SingleSelectFieldOption____refetch>;

import { ResolverParameterType as field_ProjectV2View____refetch } from './ProjectV2View/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2View.__refetch', T>
        ): IdentityWithParam<field_ProjectV2View____refetch>;

import { ResolverParameterType as field_ProjectV2Workflow____refetch } from './ProjectV2Workflow/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ProjectV2Workflow.__refetch', T>
        ): IdentityWithParam<field_ProjectV2Workflow____refetch>;

import { ResolverParameterType as field_PublicKey____refetch } from './PublicKey/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PublicKey.__refetch', T>
        ): IdentityWithParam<field_PublicKey____refetch>;

import { ResolverParameterType as field_PullRequest____refetch } from './PullRequest/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequest.__refetch', T>
        ): IdentityWithParam<field_PullRequest____refetch>;

import { ResolverParameterType as field_PullRequestCommit____refetch } from './PullRequestCommit/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequestCommit.__refetch', T>
        ): IdentityWithParam<field_PullRequestCommit____refetch>;

import { ResolverParameterType as field_PullRequestCommitCommentThread____refetch } from './PullRequestCommitCommentThread/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequestCommitCommentThread.__refetch', T>
        ): IdentityWithParam<field_PullRequestCommitCommentThread____refetch>;

import { ResolverParameterType as field_PullRequestReview____refetch } from './PullRequestReview/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequestReview.__refetch', T>
        ): IdentityWithParam<field_PullRequestReview____refetch>;

import { ResolverParameterType as field_PullRequestReviewComment____refetch } from './PullRequestReviewComment/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequestReviewComment.__refetch', T>
        ): IdentityWithParam<field_PullRequestReviewComment____refetch>;

import { ResolverParameterType as field_PullRequestReviewThread____refetch } from './PullRequestReviewThread/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequestReviewThread.__refetch', T>
        ): IdentityWithParam<field_PullRequestReviewThread____refetch>;

import { ResolverParameterType as field_PullRequestThread____refetch } from './PullRequestThread/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequestThread.__refetch', T>
        ): IdentityWithParam<field_PullRequestThread____refetch>;

import { ResolverParameterType as field_Push____refetch } from './Push/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Push.__refetch', T>
        ): IdentityWithParam<field_Push____refetch>;

import { ResolverParameterType as field_PushAllowance____refetch } from './PushAllowance/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PushAllowance.__refetch', T>
        ): IdentityWithParam<field_PushAllowance____refetch>;

import { ResolverParameterType as field_Reactable____refetch } from './Reactable/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Reactable.__refetch', T>
        ): IdentityWithParam<field_Reactable____refetch>;

import { ResolverParameterType as field_Reaction____refetch } from './Reaction/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Reaction.__refetch', T>
        ): IdentityWithParam<field_Reaction____refetch>;

import { ResolverParameterType as field_ReadyForReviewEvent____refetch } from './ReadyForReviewEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ReadyForReviewEvent.__refetch', T>
        ): IdentityWithParam<field_ReadyForReviewEvent____refetch>;

import { ResolverParameterType as field_Ref____refetch } from './Ref/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Ref.__refetch', T>
        ): IdentityWithParam<field_Ref____refetch>;

import { ResolverParameterType as field_ReferencedEvent____refetch } from './ReferencedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ReferencedEvent.__refetch', T>
        ): IdentityWithParam<field_ReferencedEvent____refetch>;

import { ResolverParameterType as field_Release____refetch } from './Release/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Release.__refetch', T>
        ): IdentityWithParam<field_Release____refetch>;

import { ResolverParameterType as field_ReleaseAsset____refetch } from './ReleaseAsset/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ReleaseAsset.__refetch', T>
        ): IdentityWithParam<field_ReleaseAsset____refetch>;

import { ResolverParameterType as field_RemovedFromMergeQueueEvent____refetch } from './RemovedFromMergeQueueEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RemovedFromMergeQueueEvent.__refetch', T>
        ): IdentityWithParam<field_RemovedFromMergeQueueEvent____refetch>;

import { ResolverParameterType as field_RemovedFromProjectEvent____refetch } from './RemovedFromProjectEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RemovedFromProjectEvent.__refetch', T>
        ): IdentityWithParam<field_RemovedFromProjectEvent____refetch>;

import { ResolverParameterType as field_RenamedTitleEvent____refetch } from './RenamedTitleEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RenamedTitleEvent.__refetch', T>
        ): IdentityWithParam<field_RenamedTitleEvent____refetch>;

import { ResolverParameterType as field_ReopenedEvent____refetch } from './ReopenedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ReopenedEvent.__refetch', T>
        ): IdentityWithParam<field_ReopenedEvent____refetch>;

import { ResolverParameterType as field_RepoAccessAuditEntry____refetch } from './RepoAccessAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoAccessAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoAccessAuditEntry____refetch>;

import { ResolverParameterType as field_RepoAddMemberAuditEntry____refetch } from './RepoAddMemberAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoAddMemberAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoAddMemberAuditEntry____refetch>;

import { ResolverParameterType as field_RepoAddTopicAuditEntry____refetch } from './RepoAddTopicAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoAddTopicAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoAddTopicAuditEntry____refetch>;

import { ResolverParameterType as field_RepoArchivedAuditEntry____refetch } from './RepoArchivedAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoArchivedAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoArchivedAuditEntry____refetch>;

import { ResolverParameterType as field_RepoChangeMergeSettingAuditEntry____refetch } from './RepoChangeMergeSettingAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoChangeMergeSettingAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoChangeMergeSettingAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigDisableAnonymousGitAccessAuditEntry____refetch } from './RepoConfigDisableAnonymousGitAccessAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigDisableAnonymousGitAccessAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigDisableAnonymousGitAccessAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigDisableCollaboratorsOnlyAuditEntry____refetch } from './RepoConfigDisableCollaboratorsOnlyAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigDisableCollaboratorsOnlyAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigDisableCollaboratorsOnlyAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigDisableContributorsOnlyAuditEntry____refetch } from './RepoConfigDisableContributorsOnlyAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigDisableContributorsOnlyAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigDisableContributorsOnlyAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigDisableSockpuppetDisallowedAuditEntry____refetch } from './RepoConfigDisableSockpuppetDisallowedAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigDisableSockpuppetDisallowedAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigDisableSockpuppetDisallowedAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigEnableAnonymousGitAccessAuditEntry____refetch } from './RepoConfigEnableAnonymousGitAccessAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigEnableAnonymousGitAccessAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigEnableAnonymousGitAccessAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigEnableCollaboratorsOnlyAuditEntry____refetch } from './RepoConfigEnableCollaboratorsOnlyAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigEnableCollaboratorsOnlyAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigEnableCollaboratorsOnlyAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigEnableContributorsOnlyAuditEntry____refetch } from './RepoConfigEnableContributorsOnlyAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigEnableContributorsOnlyAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigEnableContributorsOnlyAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigEnableSockpuppetDisallowedAuditEntry____refetch } from './RepoConfigEnableSockpuppetDisallowedAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigEnableSockpuppetDisallowedAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigEnableSockpuppetDisallowedAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigLockAnonymousGitAccessAuditEntry____refetch } from './RepoConfigLockAnonymousGitAccessAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigLockAnonymousGitAccessAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigLockAnonymousGitAccessAuditEntry____refetch>;

import { ResolverParameterType as field_RepoConfigUnlockAnonymousGitAccessAuditEntry____refetch } from './RepoConfigUnlockAnonymousGitAccessAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoConfigUnlockAnonymousGitAccessAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoConfigUnlockAnonymousGitAccessAuditEntry____refetch>;

import { ResolverParameterType as field_RepoCreateAuditEntry____refetch } from './RepoCreateAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoCreateAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoCreateAuditEntry____refetch>;

import { ResolverParameterType as field_RepoDestroyAuditEntry____refetch } from './RepoDestroyAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoDestroyAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoDestroyAuditEntry____refetch>;

import { ResolverParameterType as field_RepoRemoveMemberAuditEntry____refetch } from './RepoRemoveMemberAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoRemoveMemberAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoRemoveMemberAuditEntry____refetch>;

import { ResolverParameterType as field_RepoRemoveTopicAuditEntry____refetch } from './RepoRemoveTopicAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepoRemoveTopicAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepoRemoveTopicAuditEntry____refetch>;

import { ResolverParameterType as field_Repository____refetch } from './Repository/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Repository.__refetch', T>
        ): IdentityWithParam<field_Repository____refetch>;

import { ResolverParameterType as field_RepositoryInvitation____refetch } from './RepositoryInvitation/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryInvitation.__refetch', T>
        ): IdentityWithParam<field_RepositoryInvitation____refetch>;

import { ResolverParameterType as field_RepositoryMigration____refetch } from './RepositoryMigration/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryMigration.__refetch', T>
        ): IdentityWithParam<field_RepositoryMigration____refetch>;

import { ResolverParameterType as field_RepositoryOwner____refetch } from './RepositoryOwner/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryOwner.__refetch', T>
        ): IdentityWithParam<field_RepositoryOwner____refetch>;

import { ResolverParameterType as field_RepositoryRule____refetch } from './RepositoryRule/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryRule.__refetch', T>
        ): IdentityWithParam<field_RepositoryRule____refetch>;

import { ResolverParameterType as field_RepositoryRuleset____refetch } from './RepositoryRuleset/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryRuleset.__refetch', T>
        ): IdentityWithParam<field_RepositoryRuleset____refetch>;

import { ResolverParameterType as field_RepositoryRulesetBypassActor____refetch } from './RepositoryRulesetBypassActor/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryRulesetBypassActor.__refetch', T>
        ): IdentityWithParam<field_RepositoryRulesetBypassActor____refetch>;

import { ResolverParameterType as field_RepositoryTopic____refetch } from './RepositoryTopic/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryTopic.__refetch', T>
        ): IdentityWithParam<field_RepositoryTopic____refetch>;

import { ResolverParameterType as field_RepositoryVisibilityChangeDisableAuditEntry____refetch } from './RepositoryVisibilityChangeDisableAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryVisibilityChangeDisableAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepositoryVisibilityChangeDisableAuditEntry____refetch>;

import { ResolverParameterType as field_RepositoryVisibilityChangeEnableAuditEntry____refetch } from './RepositoryVisibilityChangeEnableAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryVisibilityChangeEnableAuditEntry.__refetch', T>
        ): IdentityWithParam<field_RepositoryVisibilityChangeEnableAuditEntry____refetch>;

import { ResolverParameterType as field_RepositoryVulnerabilityAlert____refetch } from './RepositoryVulnerabilityAlert/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field RepositoryVulnerabilityAlert.__refetch', T>
        ): IdentityWithParam<field_RepositoryVulnerabilityAlert____refetch>;

import { ResolverParameterType as field_ReviewDismissalAllowance____refetch } from './ReviewDismissalAllowance/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ReviewDismissalAllowance.__refetch', T>
        ): IdentityWithParam<field_ReviewDismissalAllowance____refetch>;

import { ResolverParameterType as field_ReviewDismissedEvent____refetch } from './ReviewDismissedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ReviewDismissedEvent.__refetch', T>
        ): IdentityWithParam<field_ReviewDismissedEvent____refetch>;

import { ResolverParameterType as field_ReviewRequest____refetch } from './ReviewRequest/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ReviewRequest.__refetch', T>
        ): IdentityWithParam<field_ReviewRequest____refetch>;

import { ResolverParameterType as field_ReviewRequestRemovedEvent____refetch } from './ReviewRequestRemovedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ReviewRequestRemovedEvent.__refetch', T>
        ): IdentityWithParam<field_ReviewRequestRemovedEvent____refetch>;

import { ResolverParameterType as field_ReviewRequestedEvent____refetch } from './ReviewRequestedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field ReviewRequestedEvent.__refetch', T>
        ): IdentityWithParam<field_ReviewRequestedEvent____refetch>;

import { ResolverParameterType as field_SavedReply____refetch } from './SavedReply/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field SavedReply.__refetch', T>
        ): IdentityWithParam<field_SavedReply____refetch>;

import { ResolverParameterType as field_SecurityAdvisory____refetch } from './SecurityAdvisory/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field SecurityAdvisory.__refetch', T>
        ): IdentityWithParam<field_SecurityAdvisory____refetch>;

import { ResolverParameterType as field_SponsorsActivity____refetch } from './SponsorsActivity/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field SponsorsActivity.__refetch', T>
        ): IdentityWithParam<field_SponsorsActivity____refetch>;

import { ResolverParameterType as field_SponsorsListing____refetch } from './SponsorsListing/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field SponsorsListing.__refetch', T>
        ): IdentityWithParam<field_SponsorsListing____refetch>;

import { ResolverParameterType as field_SponsorsListingFeaturedItem____refetch } from './SponsorsListingFeaturedItem/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field SponsorsListingFeaturedItem.__refetch', T>
        ): IdentityWithParam<field_SponsorsListingFeaturedItem____refetch>;

import { ResolverParameterType as field_SponsorsTier____refetch } from './SponsorsTier/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field SponsorsTier.__refetch', T>
        ): IdentityWithParam<field_SponsorsTier____refetch>;

import { ResolverParameterType as field_Sponsorship____refetch } from './Sponsorship/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Sponsorship.__refetch', T>
        ): IdentityWithParam<field_Sponsorship____refetch>;

import { ResolverParameterType as field_SponsorshipNewsletter____refetch } from './SponsorshipNewsletter/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field SponsorshipNewsletter.__refetch', T>
        ): IdentityWithParam<field_SponsorshipNewsletter____refetch>;

import { ResolverParameterType as field_Starrable____refetch } from './Starrable/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Starrable.__refetch', T>
        ): IdentityWithParam<field_Starrable____refetch>;

import { ResolverParameterType as field_Status____refetch } from './Status/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Status.__refetch', T>
        ): IdentityWithParam<field_Status____refetch>;

import { ResolverParameterType as field_StatusCheckRollup____refetch } from './StatusCheckRollup/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field StatusCheckRollup.__refetch', T>
        ): IdentityWithParam<field_StatusCheckRollup____refetch>;

import { ResolverParameterType as field_StatusContext____refetch } from './StatusContext/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field StatusContext.__refetch', T>
        ): IdentityWithParam<field_StatusContext____refetch>;

import { ResolverParameterType as field_Subscribable____refetch } from './Subscribable/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Subscribable.__refetch', T>
        ): IdentityWithParam<field_Subscribable____refetch>;

import { ResolverParameterType as field_SubscribedEvent____refetch } from './SubscribedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field SubscribedEvent.__refetch', T>
        ): IdentityWithParam<field_SubscribedEvent____refetch>;

import { ResolverParameterType as field_Tag____refetch } from './Tag/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Tag.__refetch', T>
        ): IdentityWithParam<field_Tag____refetch>;

import { ResolverParameterType as field_Team____refetch } from './Team/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Team.__refetch', T>
        ): IdentityWithParam<field_Team____refetch>;

import { ResolverParameterType as field_TeamAddMemberAuditEntry____refetch } from './TeamAddMemberAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field TeamAddMemberAuditEntry.__refetch', T>
        ): IdentityWithParam<field_TeamAddMemberAuditEntry____refetch>;

import { ResolverParameterType as field_TeamAddRepositoryAuditEntry____refetch } from './TeamAddRepositoryAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field TeamAddRepositoryAuditEntry.__refetch', T>
        ): IdentityWithParam<field_TeamAddRepositoryAuditEntry____refetch>;

import { ResolverParameterType as field_TeamChangeParentTeamAuditEntry____refetch } from './TeamChangeParentTeamAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field TeamChangeParentTeamAuditEntry.__refetch', T>
        ): IdentityWithParam<field_TeamChangeParentTeamAuditEntry____refetch>;

import { ResolverParameterType as field_TeamDiscussion____refetch } from './TeamDiscussion/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field TeamDiscussion.__refetch', T>
        ): IdentityWithParam<field_TeamDiscussion____refetch>;

import { ResolverParameterType as field_TeamDiscussionComment____refetch } from './TeamDiscussionComment/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field TeamDiscussionComment.__refetch', T>
        ): IdentityWithParam<field_TeamDiscussionComment____refetch>;

import { ResolverParameterType as field_TeamRemoveMemberAuditEntry____refetch } from './TeamRemoveMemberAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field TeamRemoveMemberAuditEntry.__refetch', T>
        ): IdentityWithParam<field_TeamRemoveMemberAuditEntry____refetch>;

import { ResolverParameterType as field_TeamRemoveRepositoryAuditEntry____refetch } from './TeamRemoveRepositoryAuditEntry/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field TeamRemoveRepositoryAuditEntry.__refetch', T>
        ): IdentityWithParam<field_TeamRemoveRepositoryAuditEntry____refetch>;

import { ResolverParameterType as field_Topic____refetch } from './Topic/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Topic.__refetch', T>
        ): IdentityWithParam<field_Topic____refetch>;

import { ResolverParameterType as field_TransferredEvent____refetch } from './TransferredEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field TransferredEvent.__refetch', T>
        ): IdentityWithParam<field_TransferredEvent____refetch>;

import { ResolverParameterType as field_Tree____refetch } from './Tree/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Tree.__refetch', T>
        ): IdentityWithParam<field_Tree____refetch>;

import { ResolverParameterType as field_UnassignedEvent____refetch } from './UnassignedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field UnassignedEvent.__refetch', T>
        ): IdentityWithParam<field_UnassignedEvent____refetch>;

import { ResolverParameterType as field_UnlabeledEvent____refetch } from './UnlabeledEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field UnlabeledEvent.__refetch', T>
        ): IdentityWithParam<field_UnlabeledEvent____refetch>;

import { ResolverParameterType as field_UnlockedEvent____refetch } from './UnlockedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field UnlockedEvent.__refetch', T>
        ): IdentityWithParam<field_UnlockedEvent____refetch>;

import { ResolverParameterType as field_UnmarkedAsDuplicateEvent____refetch } from './UnmarkedAsDuplicateEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field UnmarkedAsDuplicateEvent.__refetch', T>
        ): IdentityWithParam<field_UnmarkedAsDuplicateEvent____refetch>;

import { ResolverParameterType as field_UnpinnedEvent____refetch } from './UnpinnedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field UnpinnedEvent.__refetch', T>
        ): IdentityWithParam<field_UnpinnedEvent____refetch>;

import { ResolverParameterType as field_UnsubscribedEvent____refetch } from './UnsubscribedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field UnsubscribedEvent.__refetch', T>
        ): IdentityWithParam<field_UnsubscribedEvent____refetch>;

import { ResolverParameterType as field_User____refetch } from './User/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field User.__refetch', T>
        ): IdentityWithParam<field_User____refetch>;

import { ResolverParameterType as field_UserBlockedEvent____refetch } from './UserBlockedEvent/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field UserBlockedEvent.__refetch', T>
        ): IdentityWithParam<field_UserBlockedEvent____refetch>;

import { ResolverParameterType as field_UserContentEdit____refetch } from './UserContentEdit/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field UserContentEdit.__refetch', T>
        ): IdentityWithParam<field_UserContentEdit____refetch>;

import { ResolverParameterType as field_UserStatus____refetch } from './UserStatus/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field UserStatus.__refetch', T>
        ): IdentityWithParam<field_UserStatus____refetch>;

import { ResolverParameterType as field_VerifiableDomain____refetch } from './VerifiableDomain/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field VerifiableDomain.__refetch', T>
        ): IdentityWithParam<field_VerifiableDomain____refetch>;

import { ResolverParameterType as field_Workflow____refetch } from './Workflow/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Workflow.__refetch', T>
        ): IdentityWithParam<field_Workflow____refetch>;

import { ResolverParameterType as field_WorkflowRun____refetch } from './WorkflowRun/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field WorkflowRun.__refetch', T>
        ): IdentityWithParam<field_WorkflowRun____refetch>;

import { ResolverParameterType as field_WorkflowRunFile____refetch } from './WorkflowRunFile/__refetch/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field WorkflowRunFile.__refetch', T>
        ): IdentityWithParam<field_WorkflowRunFile____refetch>;

import { ResolverParameterType as field_User__RepositoryList } from './User/RepositoryList/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field User.RepositoryList', T>
        ): IdentityWithParam<field_User__RepositoryList>;

import { ResolverParameterType as field_Query__UserPage } from './Query/UserPage/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.UserPage', T>
        ): IdentityWithParam<field_Query__UserPage>;

import { ResolverParameterType as field_Query__RepositoryPage } from './Query/RepositoryPage/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.RepositoryPage', T>
        ): IdentityWithParam<field_Query__RepositoryPage>;

import { ResolverParameterType as field_Query__HomePage } from './Query/HomePage/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.HomePage', T>
        ): IdentityWithParam<field_Query__HomePage>;

import { ResolverParameterType as field_Query__UserDetail } from './Query/UserDetail/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.UserDetail', T>
        ): IdentityWithParam<field_Query__UserDetail>;

import { ResolverParameterType as field_Repository__RepositoryLink } from './Repository/RepositoryLink/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Repository.RepositoryLink', T>
        ): IdentityWithParam<field_Repository__RepositoryLink>;

import { ResolverParameterType as field_Query__Header } from './Query/Header/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.Header', T>
        ): IdentityWithParam<field_Query__Header>;

import { ResolverParameterType as field_Query__PullRequestDetail } from './Query/PullRequestDetail/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.PullRequestDetail', T>
        ): IdentityWithParam<field_Query__PullRequestDetail>;

import { ResolverParameterType as field_User__Avatar } from './User/Avatar/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field User.Avatar', T>
        ): IdentityWithParam<field_User__Avatar>;

import { ResolverParameterType as field_Actor__UserLink } from './Actor/UserLink/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Actor.UserLink', T>
        ): IdentityWithParam<field_Actor__UserLink>;

import { ResolverParameterType as field_Query__PullRequest } from './Query/PullRequest/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.PullRequest', T>
        ): IdentityWithParam<field_Query__PullRequest>;

import { ResolverParameterType as field_PullRequest__createdAtFormatted } from './PullRequest/createdAtFormatted/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequest.createdAtFormatted', T>
        ): IdentityWithParam<field_PullRequest__createdAtFormatted>;

import { ResolverParameterType as field_PullRequestConnection__PullRequestTable } from './PullRequestConnection/PullRequestTable/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequestConnection.PullRequestTable', T>
        ): IdentityWithParam<field_PullRequestConnection__PullRequestTable>;

import { ResolverParameterType as field_PullRequest__PullRequestLink } from './PullRequest/PullRequestLink/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequest.PullRequestLink', T>
        ): IdentityWithParam<field_PullRequest__PullRequestLink>;

import { ResolverParameterType as field_Query__RepositoryDetail } from './Query/RepositoryDetail/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.RepositoryDetail', T>
        ): IdentityWithParam<field_Query__RepositoryDetail>;

import { ResolverParameterType as field_IssueComment__formattedCommentCreationDate } from './IssueComment/formattedCommentCreationDate/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field IssueComment.formattedCommentCreationDate', T>
        ): IdentityWithParam<field_IssueComment__formattedCommentCreationDate>;

import { ResolverParameterType as field_PullRequest__CommentList } from './PullRequest/CommentList/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field PullRequest.CommentList', T>
        ): IdentityWithParam<field_PullRequest__CommentList>;

import { ResolverParameterType as field_Query__HomePageList } from './Query/HomePageList/reader.ts'
export function iso<T>(
            param: T & MatchesWhitespaceAndString<'field Query.HomePageList', T>
        ): IdentityWithParam<field_Query__HomePageList>;
