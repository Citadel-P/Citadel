using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Moq;
using System.Net;
using System.Net.Http.Json;
using System.Text.Json;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Alerters;

public sealed class AlertEventTests : IntegrationTestBase
{
    private readonly Mock<IAlertEventStreamManager> _alertEventStreamManager = new();
    private readonly Guid _resourceId = Guid.Parse("11111111-1111-1111-1111-111111111111");
    private readonly Guid _secondResourceId = Guid.Parse("33333333-3333-3333-3333-333333333333");
    private readonly Guid _resolvedResourceId = Guid.Parse("22222222-2222-2222-2222-222222222222");
    private readonly Guid _alertId = Guid.Parse("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa");
    private readonly Guid _secondAlertId = Guid.Parse("cccccccc-cccc-cccc-cccc-cccccccccccc");
    private readonly Guid _resolvedAlertId = Guid.Parse("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb");
    private string _resourceName = string.Empty;
    private Guid _alertRuleId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IAlertEventStreamManager>();
        services.AddSingleton(_ => _alertEventStreamManager.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        _resourceName = platform.Name;
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);

        _alertRuleId = (await uow.AlertRules.GetAllAsync(TestContext.Current.CancellationToken))
            .First(x => x.Type == AlertType.PlatformUnreachable)
            .Id;

        var activeAlert = AlertEvent.FromPersistence(
            id: _alertId,
            alertRuleId: _alertRuleId,
            type: AlertType.PlatformUnreachable,
            severity: AlertSeverity.Critical,
            info: new PlatformUnreachableAlertInfo(platform.Name, _resourceId, platform.Address),
            resourceId: _resourceId,
            resourceName: platform.Name,
            resourceType: AlertResourceType.Platform,
            deduplicationKey: "DEDUP-ACTIVE",
            openIncidentKey: "DEDUP-ACTIVE",
            acknowledgedByActorId: null,
            acknowledgedAt: null,
            resolvedByActorId: null,
            resolvedAt: null,
            resolutionNote: null,
            createdAt: new DateTime(2026, 01, 01, 10, 00, 00, DateTimeKind.Utc),
            updatedAt: new DateTime(2026, 01, 01, 10, 00, 00, DateTimeKind.Utc));

        var secondActiveAlert = AlertEvent.FromPersistence(
            id: _secondAlertId,
            alertRuleId: _alertRuleId,
            type: AlertType.PlatformUnreachable,
            severity: AlertSeverity.Warning,
            info: new PlatformUnreachableAlertInfo(platform.Name, _secondResourceId, platform.Address),
            resourceId: _secondResourceId,
            resourceName: platform.Name,
            resourceType: AlertResourceType.Platform,
            deduplicationKey: "DEDUP-ACTIVE-2",
            openIncidentKey: "DEDUP-ACTIVE-2",
            acknowledgedByActorId: null,
            acknowledgedAt: null,
            resolvedByActorId: null,
            resolvedAt: null,
            resolutionNote: null,
            createdAt: new DateTime(2026, 01, 01, 10, 30, 00, DateTimeKind.Utc),
            updatedAt: new DateTime(2026, 01, 01, 10, 30, 00, DateTimeKind.Utc));

        var resolvedAlert = AlertEvent.FromPersistence(
            id: _resolvedAlertId,
            alertRuleId: _alertRuleId,
            type: AlertType.PlatformUnreachable,
            severity: AlertSeverity.Warning,
            info: new PlatformUnreachableAlertInfo(platform.Name, _resolvedResourceId, platform.Address),
            resourceId: _resolvedResourceId,
            resourceName: platform.Name,
            resourceType: AlertResourceType.Platform,
            deduplicationKey: "DEDUP-RESOLVED",
            openIncidentKey: null,
            acknowledgedByActorId: Constants.SystemId,
            acknowledgedAt: new DateTime(2026, 01, 01, 11, 00, 00, DateTimeKind.Utc),
            resolvedByActorId: Constants.SystemId,
            resolvedAt: new DateTime(2026, 01, 01, 12, 00, 00, DateTimeKind.Utc),
            resolutionNote: "resolved during seed",
            createdAt: new DateTime(2026, 01, 01, 09, 00, 00, DateTimeKind.Utc),
            updatedAt: new DateTime(2026, 01, 01, 12, 00, 00, DateTimeKind.Utc));

        await uow.AlertEvents.AddAsync(activeAlert, TestContext.Current.CancellationToken);
        await uow.AlertEvents.AddAsync(secondActiveAlert, TestContext.Current.CancellationToken);
        await uow.AlertEvents.AddAsync(resolvedAlert, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task Get_AlertEvent_ReturnsSuccess()
    {
        var response = await Client.GetAsync($"/api/v1/alertEvents/{_alertId}", TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(response);
        var root = document.RootElement;

        Assert.Equal(_alertId, GetProperty(root, "id").GetGuid());
        Assert.Equal(_alertRuleId, GetProperty(root, "alertRuleId").GetGuid());
        Assert.Equal(_resourceId, GetProperty(root, "resourceId").GetGuid());
        Assert.Equal(_resourceName, GetProperty(root, "resourceName").GetString());
        Assert.Equal($"/platforms/{_resourceId}", GetProperty(root, "resourcePath").GetString());
    }

    [Fact]
    public async Task List_AlertEvents_UnresolvedOnly_ReturnsOnlyActiveAlerts()
    {
        var response = await Client.GetAsync("/api/v1/alertEvents?unresolvedOnly=true&page=1&pageSize=10", TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(response);
        var pagedResult = GetProperty(document.RootElement, "pagedResult");
        var items = GetProperty(pagedResult, "items");
        var ids = items.EnumerateArray().Select(x => GetProperty(x, "id").GetGuid()).ToHashSet();

        Assert.Equal(2, items.GetArrayLength());
        Assert.Equal(2, GetProperty(pagedResult, "totalCount").GetInt32());
        Assert.Contains(_alertId, ids);
        Assert.Contains(_secondAlertId, ids);
    }

    [Fact]
    public async Task Get_UnresolvedAlertEventsCount_ReturnsSuccess()
    {
        var response = await Client.GetAsync("/api/v1/alertEvents/unresolved-count", TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        using var document = await ReadJsonAsync(response);
        Assert.Equal(2, GetProperty(document.RootElement, "count").GetInt32());
    }

    [Fact]
    public async Task Get_ResolvedAlertEvent_ReturnsActor()
    {
        var response = await Client.GetAsync($"/api/v1/alertEvents/{_resolvedAlertId}", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = await uow.Actors.GetById(Constants.SystemId, TestContext.Current.CancellationToken);

        using var document = await ReadJsonAsync(response);
        var root = document.RootElement;

        Assert.Equal(Constants.SystemId, GetProperty(root, "actorId").GetGuid());
        Assert.Equal(actor?.Name, GetProperty(root, "actorName").GetString());
        Assert.Equal(actor?.Type.ToString(), GetProperty(root, "actorType").GetString());
    }

    [Fact]
    public async Task Acknowledge_AlertEvents_UpdatesState()
    {
        var content = JsonContent.Create(new { ids = new[] { _alertId, _secondAlertId } });
        var response = await Client.PostAsync("/api/v1/alertEvents/acknowledge", content, TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await uow.AlertEvents.GetByIdAsync(_alertId, TestContext.Current.CancellationToken);
        var secondPersisted = await uow.AlertEvents.GetByIdAsync(_secondAlertId, TestContext.Current.CancellationToken);

        Assert.NotNull(persisted);
        Assert.Equal(AlertEventStatus.Acknowledged, persisted!.Status);
        Assert.Equal(Constants.SystemId, persisted.AcknowledgedByActorId);
        Assert.NotNull(persisted.AcknowledgedAt);
        Assert.NotNull(secondPersisted);
        Assert.Equal(AlertEventStatus.Acknowledged, secondPersisted!.Status);
        Assert.Equal(Constants.SystemId, secondPersisted.AcknowledgedByActorId);

        await WaitForAlertStreamVerificationAsync(
            () => _alertEventStreamManager.Verify(
                x => x.SendUpdatedAlertEvents(It.Is<IEnumerable<AlertEvent>>(events => events.Count() == 2)),
                Times.Once),
            TimeSpan.FromSeconds(5),
            TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task Resolve_AlertEvents_UpdatesState()
    {
        var content = JsonContent.Create(new { ids = new[] { _alertId, _secondAlertId }, resolutionNote = "resolved from endpoint" });
        var response = await Client.PostAsync("/api/v1/alertEvents/resolve", content, TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var persisted = await uow.AlertEvents.GetByIdAsync(_alertId, TestContext.Current.CancellationToken);
        var secondPersisted = await uow.AlertEvents.GetByIdAsync(_secondAlertId, TestContext.Current.CancellationToken);

        Assert.NotNull(persisted);
        Assert.Equal(AlertEventStatus.Resolved, persisted!.Status);
        Assert.Null(persisted.OpenIncidentKey);
        Assert.Equal(Constants.SystemId, persisted.ResolvedByActorId);
        Assert.Equal("resolved from endpoint", persisted.ResolutionNote);
        Assert.NotNull(secondPersisted);
        Assert.Equal(AlertEventStatus.Resolved, secondPersisted!.Status);
        Assert.Null(secondPersisted.OpenIncidentKey);
        Assert.Equal(Constants.SystemId, secondPersisted.ResolvedByActorId);
        Assert.Equal("resolved from endpoint", secondPersisted.ResolutionNote);

        await WaitForAlertStreamVerificationAsync(
            () =>
            {
                _alertEventStreamManager.Verify(
                    x => x.SendUpdatedAlertEvents(It.Is<IEnumerable<AlertEvent>>(events => events.Count() == 2)),
                    Times.Once);
                _alertEventStreamManager.Verify(x => x.SendUnresolvedAlertCount(0), Times.Once);
            },
            TimeSpan.FromSeconds(5),
            TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task Acknowledge_ResolvedAlertEvents_ReturnsBadRequest()
    {
        var content = JsonContent.Create(new { ids = new[] { _resolvedAlertId } });
        var response = await Client.PostAsync("/api/v1/alertEvents/acknowledge", content, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    private static async Task<JsonDocument> ReadJsonAsync(HttpResponseMessage response)
        => await JsonDocument.ParseAsync(await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken), cancellationToken: TestContext.Current.CancellationToken);

    private static JsonElement GetProperty(JsonElement element, string propertyName)
    {
        foreach (var property in element.EnumerateObject())
        {
            if (string.Equals(property.Name, propertyName, StringComparison.OrdinalIgnoreCase))
                return property.Value;
        }

        throw new InvalidOperationException($"Property '{propertyName}' was not found.");
    }

    private async Task WaitForAlertStreamVerificationAsync(Action verification, TimeSpan timeout, CancellationToken cancellationToken)
    {
        var start = DateTime.UtcNow;

        while (DateTime.UtcNow - start < timeout)
        {
            try
            {
                verification();
                return;
            }
            catch (MockException)
            {
                await Task.Delay(100, cancellationToken);
            }
        }

        verification();
    }
}
