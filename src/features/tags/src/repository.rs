use crate::*;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use uuid::Uuid;
pub trait TagRepository: Send + Sync {
    /// Map the supplied literals to their comparison keys using the database collation.
    /// This operation does not look up tags or disclose whether they exist.
    fn comparison_keys<'a>(
        &'a self,
        names: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<String>, TagError>>;

    fn list_tags<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<Tag>, TagError>>;

    fn create_tag<'a>(
        &'a self,
        actor_id: ActorId,
        tag: &'a NewTag,
    ) -> BoxFuture<'a, Result<Tag, TagError>>;

    fn update_tag<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        patch: &'a TagPatch,
    ) -> BoxFuture<'a, Result<(Tag, Vec<TaggableResourceType>), TagError>>;

    fn delete_tag<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<TaggableResourceType>, TagError>>;

    fn get_resource_tags<'a>(
        &'a self,
        resource_type: TaggableResourceType,
        resource_id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<TagSummary>, TagError>>;

    fn replace_resource_tags<'a>(
        &'a self,
        actor_id: ActorId,
        resource_type: TaggableResourceType,
        resource_id: Uuid,
        tag_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<TagSummary>, TagError>>;
}
