use sqlx::{Postgres, Transaction};
use uuid::Uuid;

// Application.Features.Identity.Setup.DefaultAutomationActions. Inserted in the
// same setup transaction, never at every startup or as a second schema seed.
pub(super) async fn insert(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
) -> Result<(), sqlx::Error> {
    let system = Uuid::from_u128(1);
    let system_tag = Uuid::from_u128(0x40000000000000000000000000000001);
    let prod_tag = Uuid::from_u128(0x40000000000000000000000000000002);
    for (name, description, code, cron, tags) in [
        (
            "Prune images",
            "Prunes unused Docker images on every platform.",
            PRUNE_IMAGES,
            "0 12 * * *",
            vec![system_tag],
        ),
        (
            "Restart unhealthy stacks",
            "Restarts stacks tagged Prod when their current release is not healthy.",
            RESTART_STACKS,
            "*/15 * * * *",
            vec![system_tag, prod_tag],
        ),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO actions(id,name,description,code,schedulecron,createdbyactorid,runasactorid,alertonfailure,enabled,scheduleenabled,scheduletimezone,timeoutseconds) VALUES($1,$2,$3,$4,$5,$6,$7,true,false,false,'UTC',300)")
            .bind(id).bind(name).bind(description).bind(code).bind(cron).bind(system).bind(actor)
            .execute(&mut **tx).await?;
        for tag in tags {
            // The FK fails setup atomically if required baseline tags are absent.
            sqlx::query("INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) VALUES('AutomationAction',$1,$2,$3)")
                .bind(id).bind(tag).bind(system).execute(&mut **tx).await?;
        }
    }
    Ok(())
}

const PRUNE_IMAGES: &str = r#"const platformsResponse = await citadel.platforms.listPlatforms();
const platforms = platformsResponse?.platforms ?? [];
let pruned = 0;
let reclaimedBytes = 0;

for (const platform of platforms) {
  if (platform.status === 'Offline') continue;
  const result = await citadel.platforms.prunePlatform(platform.id, { resource: "Image" });
  const imagesDeleted = result?.imagesDeleted ?? [];
  const reclaimed = Number(result?.spaceReclaimed ?? 0);
  reclaimedBytes += reclaimed;
  pruned += imagesDeleted.length;
  if (imagesDeleted.length === 0) {
    console.log(`No unused images on ${platform.name}.`);
    continue;
  }
  console.log(`Pruned ${imagesDeleted.length} image item(s) on ${platform.name}; reclaimed ${reclaimed} bytes.`);
}
console.log(`Pruned ${pruned} image item(s); reclaimed ${reclaimedBytes} bytes.`);
"#;

const RESTART_STACKS: &str = r#"const stacksResponse = await citadel.stacks.listStacks({ tags: ["Prod"] });
const stacks = stacksResponse?.stacks ?? [];
const unhealthyStacks = stacks.filter(
  (stack) => stack.status !== "Healthy" && stack.controlState !== "Processing"
);
if (unhealthyStacks.length === 0) {
  console.log("No unhealthy Prod stacks found.");
} else {
  const stackIds = unhealthyStacks.map((stack) => stack.id);
  await citadel.stacks.restartStacks(stackIds);
  console.log(`Requested restart for ${stackIds.length} Prod stack(s).`);
}
"#;
