using System.Net.Http.Json;
using System.Text.Json;
using Domain.Contracts.Interfaces;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Actors;

public sealed class ActorEndpointTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task ActorEndpoints_ShouldReadAndPersistEnabledState()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var subject = await CreateAuthorizationSubjectAsync();

        using var getResponse = await Client.GetAsync($"/api/v1/actors/{subject.ActorId:D}", cancellationToken);
        getResponse.EnsureSuccessStatusCode();
        using var getDocument = JsonDocument.Parse(await getResponse.Content.ReadAsStreamAsync(cancellationToken));
        Assert.Equal(subject.ActorId, getDocument.RootElement.GetProperty("id").GetGuid());
        Assert.True(getDocument.RootElement.GetProperty("isEnabled").GetBoolean());

        using var patchResponse = await Client.PatchAsJsonAsync(
            $"/api/v1/actors/{subject.ActorId:D}/enabled",
            new { isEnabled = false },
            cancellationToken);
        patchResponse.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var persisted = await scope.ServiceProvider.GetRequiredService<IUnitOfWork>()
            .Actors.GetById(subject.ActorId, cancellationToken);
        Assert.NotNull(persisted);
        Assert.False(persisted.IsEnabled);
    }
}
