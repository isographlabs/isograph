use prelude::Postfix;

use crate::validation::{EntrypointNotFetchableError, SelectableKey};
use crate::{
    ClientId, CompilationProfile, EntityOrigin, EntityShape, IsographDb, NetworkProtocol,
    ProfileEntity, ProfileSelectable, SelectableKind,
};

/// The protocol's plan for fetching `entity` as the parent of the entrypoint `selectable`.
pub fn entrypoint_fetch_plan<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    entity: &ProfileEntity<TProfile>,
    selectable: &ProfileSelectable<TProfile>,
) -> Result<ProfileFetchPlan<TProfile>, ProfileEntrypointNotFetchableError<TProfile>> {
    let data = match entity.origin.reference() {
        EntityOrigin::Source => {
            return EntrypointNotFetchableError::SourceEntity(entity.name).wrap_err();
        }
        EntityOrigin::Protocol(data) => data,
    };
    if let SelectableKind::Server(_) = selectable.kind {
        return EntrypointNotFetchableError::ProtocolSelectable(SelectableKey {
            entity: selectable.parent_entity,
            selectable: selectable.name,
        })
        .wrap_err();
    }
    match TProfile::NetworkProtocol::entity_shape(data) {
        EntityShape::Composite(_) => {}
        EntityShape::Leaf | EntityShape::Input => {
            return EntrypointNotFetchableError::NotComposite(entity.name).wrap_err();
        }
    }
    TProfile::NetworkProtocol::fetch_plan(db, client, entity.name)
        .map_err(EntrypointNotFetchableError::Protocol)
}

pub type ProfileFetchPlan<TProfile> =
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::FetchPlan;
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
    use crate::validation::EntrypointNotFetchableError;
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
            EntrypointNotFetchableError::SourceEntity(owner).wrap_err()
        );
    }
}
