using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Text.Json;
using System.Text;

namespace Tests.Integration.Application.Features.Alerters;

public class AlertChannelViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task List_AlertChannels_Should_Return_Only_Channels_User_Is_Permitted_To_View()
    {
        var visibleChannelId = await CreateAlertChannelAsync("channel-visible");
        await CreateAlertChannelAsync("channel-hidden");

        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.AlertChannel, visibleChannelId, ResourceAction.View)]);

        Client.DefaultRequestHeaders.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/alertRules/channels", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.GetProperty("channels");
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(visibleChannelId, items[0].GetProperty("id").GetGuid());
        Assert.Equal("channel-visible", items[0].GetProperty("name").GetString());
    }

    private async Task<Guid> CreateAlertChannelAsync(string name)
    {
        var createJson = $$"""
        {
          "name": "{{name}}",
          "alertDestination": "Discord",
          "url": "https://discord.com/api/webhooks/test"
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/alertRules/channels",
            new StringContent(createJson, Encoding.UTF8, "application/json"),
            cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        return (await uow.AlertRules.GetAllChannelsAsync(TestContext.Current.CancellationToken))
            .Single(x => x.Name == name)
            .Id;
    }
}
