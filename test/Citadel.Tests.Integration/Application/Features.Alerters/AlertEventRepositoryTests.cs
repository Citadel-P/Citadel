using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Alerters;

public sealed class AlertEventRepositoryTests : IntegrationTestBase
{
    private readonly Guid _firstResourceId = Guid.Parse("44444444-4444-4444-4444-444444444444");
    private readonly Guid _secondResourceId = Guid.Parse("55555555-5555-5555-5555-555555555555");
    private Guid _alertRuleId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        _alertRuleId = (await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken))
            .First(x => x.Type == AlertType.PlatformUnreachable)
            .Id;
    }

    [Fact]
    public async Task AddAsync_Should_Deduplicate_Unresolved_AlertEvents()
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var resourceId = Guid.NewGuid();

        var first = CreateAlertEvent(resourceId, "platform-1", "https://platform-1");
        var second = CreateAlertEvent(resourceId, "platform-1", "https://platform-1");

        await uow.AlertEvents.AddAsync(first, TestContext.Current.CancellationToken);
        await uow.AlertEvents.AddAsync(second, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var events = await uow.AlertEvents.GetPagedAsync(resourceId, AlertType.PlatformUnreachable, AlertResourceType.Platform, 1, 10, TestContext.Current.CancellationToken);
        var unresolvedCount = await uow.AlertEvents.CountUnresolvedAsync(TestContext.Current.CancellationToken);

        Assert.Single(events.Items);
        Assert.Equal(1, unresolvedCount);
    }

    [Fact]
    public async Task UpdateAsync_Should_Persist_Acknowledge_And_Resolve_State()
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var resourceId = Guid.NewGuid();
        var alertEvent = CreateAlertEvent(resourceId, "platform-2", "https://platform-2");

        await uow.AlertEvents.AddAsync(alertEvent, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var now = DateTime.UtcNow;
        alertEvent.Acknowledge(Constants.SystemId, now);
        alertEvent.Resolve(Constants.SystemId, now, "resolved manually");

        await uow.AlertEvents.UpdateAsync(alertEvent, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var persisted = await uow.AlertEvents.GetByIdAsync(alertEvent.Id, TestContext.Current.CancellationToken);
        var unresolvedCount = await uow.AlertEvents.CountUnresolvedAsync(TestContext.Current.CancellationToken);

        Assert.NotNull(persisted);
        Assert.Equal(AlertEventStatus.Resolved, persisted!.Status);
        Assert.Null(persisted.OpenIncidentKey);
        Assert.Equal(Constants.SystemId, persisted.AcknowledgedByActorId);
        Assert.Equal(Constants.SystemId, persisted.ResolvedByActorId);
        Assert.Equal("resolved manually", persisted.ResolutionNote);
        Assert.Equal(now, persisted.UpdatedAt);
        Assert.Equal(0, unresolvedCount);
    }

    [Fact]
    public async Task GetByIdAsync_With_Multiple_Ids_Should_Return_All_Matching_AlertEvents()
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var first = CreateAlertEvent(_firstResourceId, "platform-3", "https://platform-3");
        var second = CreateAlertEvent(_secondResourceId, "platform-4", "https://platform-4");

        await uow.AlertEvents.AddAsync(first, TestContext.Current.CancellationToken);
        await uow.AlertEvents.AddAsync(second, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var events = (await uow.AlertEvents.GetByIdAsync([first.Id, second.Id], TestContext.Current.CancellationToken)).ToList();

        Assert.Equal(2, events.Count);
        Assert.Contains(events, x => x.Id == first.Id);
        Assert.Contains(events, x => x.Id == second.Id);
    }

    [Fact]
    public async Task BulkUpdateAsync_Should_Persist_State_For_Multiple_AlertEvents()
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();

        var first = CreateAlertEvent(_firstResourceId, "platform-5", "https://platform-5");
        var second = CreateAlertEvent(_secondResourceId, "platform-6", "https://platform-6");

        await uow.AlertEvents.AddAsync(first, TestContext.Current.CancellationToken);
        await uow.AlertEvents.AddAsync(second, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var now = DateTime.UtcNow;
        first.Acknowledge(Constants.SystemId, now);
        second.Resolve(Constants.SystemId, now, "bulk resolved");

        await uow.AlertEvents.BulkUpdateAsync([first, second], TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var persisted = (await uow.AlertEvents.GetByIdAsync([first.Id, second.Id], TestContext.Current.CancellationToken))
            .OrderBy(x => x.Id)
            .ToList();

        Assert.Equal(2, persisted.Count);

        var persistedAcknowledged = Assert.Single(persisted.Where(x => x.Id == first.Id));
        Assert.Equal(AlertEventStatus.Acknowledged, persistedAcknowledged.Status);
        Assert.Equal(Constants.SystemId, persistedAcknowledged.AcknowledgedByActorId);
        Assert.Equal(now, persistedAcknowledged.UpdatedAt);

        var persistedResolved = Assert.Single(persisted.Where(x => x.Id == second.Id));
        Assert.Equal(AlertEventStatus.Resolved, persistedResolved.Status);
        Assert.Null(persistedResolved.OpenIncidentKey);
        Assert.Equal(Constants.SystemId, persistedResolved.ResolvedByActorId);
        Assert.Equal("bulk resolved", persistedResolved.ResolutionNote);
        Assert.Equal(now, persistedResolved.UpdatedAt);
    }

    private AlertEvent CreateAlertEvent(Guid resourceId, string platformName, string address)
        => new(
            alertRuleId: _alertRuleId,
            type: AlertType.PlatformUnreachable,
            severity: AlertSeverity.Critical,
            info: new PlatformUnreachableAlertInfo(platformName, resourceId, address),
            resourceId: resourceId,
            resourceType: AlertResourceType.Platform,
            createdByActorId: Constants.SystemId);
}
