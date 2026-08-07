using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Git;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Stacks;
using Domain.Entities.Tags;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net;
using System.Text;
using System.Text.Json.Nodes;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Stacks;

public class StackDuplicateTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid _platformId;
    private Guid _sourceStackId;
    private Guid _tagId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var tag = Tag.Create("duplicate-stack", "#33AA66", Constants.SystemId);
        var stack = Stack.Create(
            name: "source-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.WebEditor,
            platformId: platform.Id,
            spec: new ManualStack(
                ComposeFile: """
                services:
                  app:
                    image: nginx:latest
                networks:
                  frontend:
                    external: true
                """,
                UpdateBehavior: StackUpdateBehavior.ServiceAutoDeploy),
            description: "source stack description");

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Tags.AddAsync(tag, TestContext.Current.CancellationToken);
        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken, [tag.Id], Constants.SystemId);
        await uow.ResourceBindings.AddAsync(
            new ResourceBinding(
                Name: "stack_var",
                Kind: ResourceBindingKind.Variable,
                Scope: ResourceBindingScope.Stack,
                ResourceId: stack.Id,
                Value: "stack-value",
                SecretId: null),
            TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        _sourceStackId = stack.Id;
        _platformId = platform.Id;
        _tagId = tag.Id;
    }

    [Fact]
    public async Task DuplicateDraft_Should_CreateStack_And_RecordDuplicateActivity()
    {
        var draftResponse = await Client.GetAsync(
            $"/api/v1/stacks/{_sourceStackId}/duplicate-draft",
            TestContext.Current.CancellationToken);
        draftResponse.EnsureSuccessStatusCode();

        var draftDocument = await ReadJsonObjectAsync(draftResponse);
        var draft = draftDocument["draft"]!.AsObject();
        var warnings = draftDocument["warnings"]!.AsArray();
        var spec = draft["spec"]!.AsObject();

        Assert.Equal("source-stack-copy", draft["name"]!.GetValue<string>());
        Assert.Equal("WebEditor", draft["stackSource"]!.GetValue<string>());
        Assert.Equal(_tagId, draft["tagIds"]!.AsArray()[0]!.GetValue<Guid>());
        Assert.Equal("WebEditor", spec["$type"]!.GetValue<string>());
        Assert.Contains("nginx:latest", spec["composeFile"]!.GetValue<string>());
        Assert.DoesNotContain(warnings, warning =>
            warning?["code"]?.GetValue<string>() == "RESOURCE_BINDINGS_NOT_COPIED");
        AssertHasWarning(warnings, "EXTERNAL_NETWORK");

        var createResponse = await Client.PostAsync(
            "/api/v1/stacks",
            JsonContent(draft.ToJsonString()),
            TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();

        var createDocument = await ReadJsonObjectAsync(createResponse);
        var createdStackId = createDocument["id"]!.GetValue<Guid>();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var created = await uow.Stacks.GetAsync(createdStackId, TestContext.Current.CancellationToken);

        Assert.NotNull(created);
        Assert.Equal("source-stack-copy", created.Name);
        Assert.Equal("source stack description", created.Description);
        Assert.Equal(StackSource.WebEditor, created.StackSource);
        Assert.Contains("nginx:latest", Assert.IsType<ManualStack>(created.CurrentStackRelease!.Spec).ComposeFile);
        Assert.Equal(_tagId, Assert.Single(created.Tags).Id);

        var copiedBinding = Assert.Single(await uow.ResourceBindings.GetEntriesAsync(
            ResourceBindingScope.Stack,
            createdStackId,
            TestContext.Current.CancellationToken));
        Assert.Equal("stack_var", copiedBinding.Name);
        Assert.Equal("stack-value", copiedBinding.Value);
        Assert.Equal(createdStackId, copiedBinding.ResourceId);
        var sourceBinding = Assert.Single(await uow.ResourceBindings.GetEntriesAsync(
            ResourceBindingScope.Stack,
            _sourceStackId,
            TestContext.Current.CancellationToken));
        Assert.NotEqual(sourceBinding.Id, copiedBinding.Id);

        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            createdStackId,
            ActivityResourceType.Stack,
            ActivityEventType.StackDuplicated,
            1,
            10,
            TestContext.Current.CancellationToken);
        var activity = await uow.ActivityEventRepository.GetByIdAsync(
            Assert.Single(activities.Items).Id,
            TestContext.Current.CancellationToken);
        var info = Assert.IsType<StackDuplicated>(activity?.Info);

        Assert.Equal(_sourceStackId, info.Source.ResourceId);
        Assert.Equal("source-stack", info.Source.ResourceName);
        Assert.Equal("source-stack-copy", info.Stack.Name);
    }

    [Fact]
    public async Task DuplicateDraft_Should_UseNextAvailableName_WhenDefaultCopyNameExists()
    {
        await AddStackAsync("source-stack-copy");

        var draftResponse = await Client.GetAsync(
            $"/api/v1/stacks/{_sourceStackId}/duplicate-draft",
            TestContext.Current.CancellationToken);
        draftResponse.EnsureSuccessStatusCode();

        var draftDocument = await ReadJsonObjectAsync(draftResponse);
        var draft = draftDocument["draft"]!.AsObject();

        Assert.Equal("source-stack-copy-2", draft["name"]!.GetValue<string>());
    }

    [Fact]
    public async Task DuplicateDraft_Should_NotCopyComposeProjectName()
    {
        var sourceId = await AddStackAsync("source-with-project-name", projectName: "source-runtime-name");

        var draftResponse = await Client.GetAsync(
            $"/api/v1/stacks/{sourceId}/duplicate-draft",
            TestContext.Current.CancellationToken);
        draftResponse.EnsureSuccessStatusCode();

        var draftDocument = await ReadJsonObjectAsync(draftResponse);
        var draft = draftDocument["draft"]!.AsObject();
        var spec = draft["spec"]!.AsObject();

        Assert.Equal("source-with-project-name-copy", draft["name"]!.GetValue<string>());
        Assert.True(!spec.TryGetPropertyValue("projectName", out var projectName) || projectName is null);
        AssertHasWarning(draftDocument["warnings"]!.AsArray(), "COMPOSE_PROJECT_NAME_NOT_COPIED");

        var createResponse = await Client.PostAsync(
            "/api/v1/stacks",
            JsonContent(draft.ToJsonString()),
            TestContext.Current.CancellationToken);
        createResponse.EnsureSuccessStatusCode();

        var createDocument = await ReadJsonObjectAsync(createResponse);
        var createdStackId = createDocument["id"]!.GetValue<Guid>();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var created = await uow.Stacks.GetAsync(createdStackId, TestContext.Current.CancellationToken);

        Assert.NotNull(created);
        Assert.Equal("source-with-project-name-copy", created.Name);
        Assert.Null(Assert.IsType<ManualStack>(created.CurrentStackRelease!.Spec).ProjectName);
    }

    [Fact]
    public async Task DuplicateDraft_ForGitStack_Should_OmitWebhookSecret_And_Warn()
    {
        var gitStackId = await AddGitStackWithWebhookSecretAsync();

        var draftResponse = await Client.GetAsync(
            $"/api/v1/stacks/{gitStackId}/duplicate-draft",
            TestContext.Current.CancellationToken);
        draftResponse.EnsureSuccessStatusCode();

        var draftDocument = await ReadJsonObjectAsync(draftResponse);
        var draft = draftDocument["draft"]!.AsObject();
        var spec = draft["spec"]!.AsObject();
        var webhook = spec["webhook"]!.AsObject();

        Assert.Equal("source-git-stack-copy", draft["name"]!.GetValue<string>());
        Assert.Equal("Git", draft["stackSource"]!.GetValue<string>());
        Assert.Equal("Git", spec["$type"]!.GetValue<string>());
        Assert.Equal("pinned-commit", spec["commitSha"]!.GetValue<string>());
        Assert.True(!webhook.TryGetPropertyValue("secret", out var secret) || secret is null);
        AssertHasWarning(draftDocument["warnings"]!.AsArray(), "WEBHOOK_SECRET_NOT_COPIED");
    }

    [Fact]
    public async Task CreateStack_WithInvalidDuplicateSourceType_Should_ReturnBadRequest_And_NotCreateStack()
    {
        var createJson = $$"""
        {
          "name": "invalid-source-type-stack",
          "platformId": "{{_platformId}}",
          "description": "invalid duplicate source type",
          "stackSource": "WebEditor",
          "spec": {
            "$type": "WebEditor",
            "composeFile": "services:\n  app:\n    image: nginx:latest\n",
            "updateBehavior": "Disabled"
          },
          "duplicateSource": {
            "resourceType": "Deployment",
            "resourceId": "{{_sourceStackId}}",
            "resourceName": "forged-source"
          }
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/stacks",
            JsonContent(createJson),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.False(await uow.Stacks.ExistsAsync(
            "invalid-source-type-stack",
            TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task CreateStack_WithMissingDuplicateSource_Should_ReturnNotFound_And_NotCreateStack()
    {
        var createJson = $$"""
        {
          "name": "missing-source-stack",
          "platformId": "{{_platformId}}",
          "description": "missing duplicate source",
          "stackSource": "WebEditor",
          "spec": {
            "$type": "WebEditor",
            "composeFile": "services:\n  app:\n    image: nginx:latest\n",
            "updateBehavior": "Disabled"
          },
          "duplicateSource": {
            "resourceType": "Stack",
            "resourceId": "{{Guid.CreateVersion7()}}",
            "resourceName": "missing-source"
          }
        }
        """;

        var response = await Client.PostAsync(
            "/api/v1/stacks",
            JsonContent(createJson),
            TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        Assert.False(await uow.Stacks.ExistsAsync(
            "missing-source-stack",
            TestContext.Current.CancellationToken));
    }

    private async Task<Guid> AddStackAsync(string name, string? projectName = null)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stack = Stack.Create(
            name: name,
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.WebEditor,
            platformId: _platformId,
            spec: new ManualStack(
                ComposeFile: "services:\n  app:\n    image: nginx:latest\n",
                UpdateBehavior: StackUpdateBehavior.Disabled,
                ProjectName: projectName));

        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        return stack.Id;
    }

    private async Task<Guid> AddGitStackWithWebhookSecretAsync()
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var repo = new GitRepository(
            name: "duplicate-git-repo",
            description: "Repository for duplicate stack tests",
            url: "https://github.com/octocat/Hello-World.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);
        var stack = Stack.Create(
            name: "source-git-stack",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: _platformId,
            spec: new GitStack(
                GitRepoId: repo.Id,
                Branch: "main",
                CommitSha: "pinned-commit",
                UpdateBehavior: StackUpdateBehavior.Notify,
                Webhook: new StackWebhookConfig(Enabled: true, Secret: "webhook-secret"),
                ComposePaths: ["compose.yml"]));

        await uow.GitRepositories.AddAsync(repo, TestContext.Current.CancellationToken);
        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return stack.Id;
    }

    private static async Task<JsonObject> ReadJsonObjectAsync(HttpResponseMessage response)
    {
        var json = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        return JsonNode.Parse(json)!.AsObject();
    }

    private static StringContent JsonContent(string json)
        => new(json, Encoding.UTF8, "application/json");

    private static void AssertHasWarning(JsonArray warnings, string code)
        => Assert.Contains(warnings, warning => warning?["code"]?.GetValue<string>() == code);
}
