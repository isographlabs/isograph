use std::collections::BTreeSet;

use common_lang_types::EntityName;
use prelude::Postfix;

use crate::validation::{EntityNotFetchableError, EntrypointNotFetchableError, SelectableKey};
use crate::{
    ClientId, CompilationProfile, CompletedIdentity, EntityOrigin, EntityShape, FixedRecordId,
    IdentitySelection, IsographDb, NetworkProtocol, ProfileEntity, ProfileSelectable,
    SelectableKind,
};

/// The root entities of one operation. Each has one record, at `FixedRecordId::root()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootEntities {
    /// The root entity whose record holds the operation's top-level selections.
    pub operation: EntityName,
    /// The other root entities.
    pub others: BTreeSet<EntityName>,
}

impl RootEntities {
    /// `Fixed(FixedRecordId::root())` when `entity` is `operation` or one of `others`;
    /// otherwise `identity`'s `Select(name)` as `Select(name)` and `Omit` as `Omit`.
    pub fn identity_of(
        &self,
        entity: EntityName,
        identity: IdentitySelection,
    ) -> CompletedIdentity {
        if entity == self.operation || self.others.contains(entity.reference()) {
            return CompletedIdentity::Fixed(FixedRecordId::root());
        }
        match identity {
            IdentitySelection::Select(name) => CompletedIdentity::Select(name),
            IdentitySelection::Omit => CompletedIdentity::Omit,
        }
    }
}

/// The protocol's plan for fetching `entity` by itself.
pub fn entity_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
) -> Result<ProfileFetchPlan<TProfile>, ProfileEntityNotFetchableError<TProfile>> {
    let data = match entity.origin.reference() {
        EntityOrigin::Source => {
            return EntityNotFetchableError::SourceEntity(entity.name).wrap_err();
        }
        EntityOrigin::Protocol(data) => data,
    };
    match TProfile::NetworkProtocol::entity_shape(data) {
        EntityShape::Composite(_) => {}
        EntityShape::Leaf | EntityShape::Input => {
            return EntityNotFetchableError::NotComposite(entity.name).wrap_err();
        }
    }
    TProfile::NetworkProtocol::fetch_plan(db, client, entity.name)
        .map_err(EntityNotFetchableError::Protocol)
}

/// The protocol's plan for fetching `entity` as the parent of the entrypoint `selectable`.
pub fn entrypoint_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
    selectable: &ProfileSelectable<TProfile>,
) -> Result<ProfileFetchPlan<TProfile>, ProfileEntrypointNotFetchableError<TProfile>> {
    if let SelectableKind::Server(_) = selectable.kind {
        return EntrypointNotFetchableError::ProtocolSelectable(SelectableKey {
            entity: selectable.parent_entity,
            selectable: selectable.name,
        })
        .wrap_err();
    }
    entity_fetch_plan(db, client, entity).map_err(EntrypointNotFetchableError::Entity)
}

pub type ProfileFetchPlan<TProfile> =
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::FetchPlan;
pub type ProfileEntityNotFetchableError<TProfile> = EntityNotFetchableError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;
pub type ProfileEntrypointNotFetchableError<TProfile> = EntrypointNotFetchableError<
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::NotFetchable,
>;

#[cfg(test)]
mod tests {
    use common_lang_types::{EntityName, SelectableName};
    use intern::string_key::Intern;
    use prelude::Postfix;

    use super::entrypoint_fetch_plan;
    use crate::test_profile::TestProfile;
    use crate::validation::{EntityNotFetchableError, EntrypointNotFetchableError};
    use crate::{
        ClientId, FileMembership, IncludeFile, IsographDb, entity_named, selectable_named,
    };

    const CLIENT: ClientId = ClientId(1);

    const SOURCE_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Skip,
    };

    #[test]
    fn a_source_entity_cannot_parent_an_entrypoint() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(
            "src/a.ts".intern().to(),
            SOURCE_FILE,
            "iso(`entity Owner`) iso(`field Owner.Badge`)".to_owned(),
        );
        let owner: EntityName = "Owner".intern().to();
        let badge: SelectableName = "Badge".intern().to();
        let entity = entity_named(db.reference(), CLIENT, owner)
            .clone()
            .expect("the fixture declares Owner");
        let selectable = selectable_named(db.reference(), CLIENT, owner, badge)
            .clone()
            .expect("the fixture declares Owner.Badge");
        assert_eq!(
            entrypoint_fetch_plan(
                db.reference(),
                CLIENT,
                entity.reference(),
                selectable.reference()
            ),
            EntrypointNotFetchableError::Entity(EntityNotFetchableError::SourceEntity(owner))
                .wrap_err()
        );
    }
}
