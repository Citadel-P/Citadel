using Domain.Contracts.Interfaces;
using Domain.Entities.Tags;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net.Http.Headers;
using System.Text.Json;

namespace Tests.Integration.Application.Features.Tags;

public class TagViewTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task List_Tags_Should_Return_Collection_Capabilities()
    {
        var subject = await CreateAuthorizationSubjectAsync(directRoleId: ViewerRoleId);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/tags", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var capabilities = document.RootElement.GetProperty("capabilities");
        Assert.True(capabilities.GetProperty("canRead").GetBoolean());
        Assert.False(capabilities.GetProperty("canWrite").GetBoolean());
        Assert.False(capabilities.GetProperty("canExecute").GetBoolean());
    }

    [Fact]
    public async Task List_Tags_Should_Filter_Overrides_And_Return_Row_Capabilities()
    {
        var visibleTagId = await CreateTagAsync($"visible-{Guid.CreateVersion7():N}");
        var hiddenTagId = await CreateTagAsync($"hidden-{Guid.CreateVersion7():N}");
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Tag, visibleTagId, PermissionLevel.Read)]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/tags", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var tags = document.RootElement.GetProperty("tags");
        var tag = Assert.Single(tags.EnumerateArray());
        Assert.Equal(visibleTagId, tag.GetProperty("id").GetGuid());
        Assert.NotEqual(hiddenTagId, tag.GetProperty("id").GetGuid());

        var rowCapabilities = tag.GetProperty("capabilities");
        Assert.True(rowCapabilities.GetProperty("canRead").GetBoolean());
        Assert.False(rowCapabilities.GetProperty("canWrite").GetBoolean());

        var collectionCapabilities = document.RootElement.GetProperty("capabilities");
        Assert.False(collectionCapabilities.GetProperty("canWrite").GetBoolean());
    }

    private async Task<Guid> CreateTagAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var tag = Tag.Create(name, "#3366ff", Constants.SystemId);
        await unitOfWork.Tags.AddAsync(tag, TestContext.Current.CancellationToken);
        await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        return tag.Id;
    }
}
