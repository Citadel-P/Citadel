use citadel_platforms::containers::ContainerTarget;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub(crate) async fn node_image(
    pool: &PgPool,
    platform: Uuid,
) -> Result<Option<(String, String)>, sqlx::Error> {
    sqlx::query_as::<_, (String, String)>("SELECT agentimagereference,agentimagedigest FROM swarmnodeagentinstallations WHERE platformid=$1 AND desiredstate='Installed'")
                .bind(platform).fetch_optional(pool).await
}

pub(crate) async fn expired(
    pool: &PgPool,
    after: Uuid,
    before: i64,
) -> Result<Vec<ContainerTarget>, sqlx::Error> {
    let rows = sqlx::query("SELECT id,platformid,dockercontainerid,dockernodeid FROM containers WHERE issystem AND systemrole='volume-helper' AND id>$1 AND created<$2 ORDER BY id LIMIT 25")
            .bind(after).bind(before).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(ContainerTarget {
                id: row.try_get("id")?,
                platform_id: row.try_get("platformid")?,
                docker_id: row.try_get("dockercontainerid")?,
                node_id: row.try_get("dockernodeid")?,
            })
        })
        .collect()
}
