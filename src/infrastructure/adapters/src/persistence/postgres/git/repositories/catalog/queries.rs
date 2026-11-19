use super::SYNC_STATUS;
use crate::persistence::postgres::authorization::AUTHORIZED_BY_LEVELS_CTE as AUTHORIZED_CTE;

pub(super) fn authorized_list() -> String {
    format!(
        r#"{AUTHORIZED_CTE}
SELECT resource.*, {SYNC_STATUS}, COALESCE((SELECT jsonb_agg(jsonb_build_object('id',tag.id,'name',tag.name,'color',tag.color) ORDER BY tag.name,tag.id)
 FROM resourcetags link JOIN tags tag ON tag.id=link.tagid WHERE link.resourcetype=$5 AND link.resourceid=resource.id),'[]'::jsonb) AS tags,
 NULL::jsonb AS latest_activity
FROM gitrepositories resource WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
 SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
 WHERE access.resourcetype=$2 AND access.resourceid=resource.id AND access.permissionlevel = ANY($3)))
ORDER BY resource.name,resource.id"#
    )
}
