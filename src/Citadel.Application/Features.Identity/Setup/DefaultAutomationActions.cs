using Domain.Entities.Automation;
using Hosting.Common;

namespace Application.Features.Identity.Setup;

internal sealed record DefaultAutomationAction(
    AutomationAction Action,
    IReadOnlyCollection<Guid> TagIds);

internal static class DefaultAutomationActions
{
    private static readonly Guid SystemTagId =
        Guid.Parse("40000000-0000-0000-0000-000000000001");
    private static readonly Guid ProdTagId =
        Guid.Parse("40000000-0000-0000-0000-000000000002");

    internal static IReadOnlyList<DefaultAutomationAction> Create(Guid administratorActorId)
    {
        var pruneImages = new AutomationAction(
            name: "Prune images",
            description: "Prunes unused Docker images on every platform.",
            code: """
                const platformsResponse = await citadel.platforms.listPlatforms();
                const platforms = platformsResponse?.platforms ?? [];
                let pruned = 0;
                let reclaimedBytes = 0;

                for (const platform of platforms) {
                  if (platform.status === 'Offline') continue
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
                """.Replace("\r\n", "\n"),
            defaultArgsJson: "{}",
            enabled: false,
            scheduleEnabled: true,
            scheduleCron: "0 12 * * *",
            scheduleTimeZone: "UTC",
            webhook: null,
            timeoutSeconds: 300,
            alertOnFailure: true,
            runAsActorId: administratorActorId,
            createdByActorId: Constants.SystemId);

        var restartUnhealthyStacks = new AutomationAction(
            name: "Restart unhealthy stacks",
            description: "Restarts stacks tagged Prod when their current release is not healthy.",
            code: """
                const stacksResponse = await citadel.stacks.listStacks({ tags: ["Prod"] });
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
                """.Replace("\r\n", "\n"),
            defaultArgsJson: "{}",
            enabled: false,
            scheduleEnabled: true,
            scheduleCron: "*/15 * * * *",
            scheduleTimeZone: "UTC",
            webhook: null,
            timeoutSeconds: 300,
            alertOnFailure: true,
            runAsActorId: administratorActorId,
            createdByActorId: Constants.SystemId);

        return
        [
            new DefaultAutomationAction(pruneImages, [SystemTagId]),
            new DefaultAutomationAction(
                restartUnhealthyStacks,
                [SystemTagId, ProdTagId])
        ];
    }
}
