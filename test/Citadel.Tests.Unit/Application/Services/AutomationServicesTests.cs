using Application.Services;
using Domain;
using Domain.Entities.Automation;

namespace Tests.Unit.Application.Services;

public sealed class AutomationServicesTests
{
    [Fact]
    public void AutomationRunCoordinator_ShouldCancelRegisteredRun()
    {
        var coordinator = new AutomationRunCoordinator();
        var runId = Guid.NewGuid();

        var cts = coordinator.Register(runId);

        Assert.True(coordinator.Cancel(runId));
        Assert.True(cts.IsCancellationRequested);
    }

    [Fact]
    public void AutomationRunCoordinator_ShouldRejectDuplicateRegistration()
    {
        var coordinator = new AutomationRunCoordinator();
        var runId = Guid.NewGuid();

        coordinator.Register(runId);

        Assert.Throws<InvalidOperationException>(() => coordinator.Register(runId));
    }

    [Fact]
    public void AutomationInputValidation_ShouldRequireJsonObject()
    {
        var success = AutomationInputValidation.ValidateJsonObject("""{"name":"value"}""", "Args");
        var array = AutomationInputValidation.ValidateJsonObject("""["value"]""", "Args");
        var invalid = AutomationInputValidation.ValidateJsonObject("{", "Args");

        Assert.True(success.IsSuccess());
        Assert.True(array.IsFailure(out var arrayError));
        Assert.Equal("Args must be a JSON object.", arrayError.Message);
        Assert.True(invalid.IsFailure(out var invalidError));
        Assert.Contains("Args is not valid JSON:", invalidError.Message);
    }

    [Fact]
    public void AutomationInputValidation_ShouldNormalizeBlankJsonToEmptyObject()
    {
        Assert.Equal("{}", AutomationInputValidation.NormalizeJsonObject(null));
        Assert.Equal("{}", AutomationInputValidation.NormalizeJsonObject("   "));
        Assert.Equal("""{"x":1}""", AutomationInputValidation.NormalizeJsonObject("""  {"x":1}  """));
    }

    [Fact]
    public void AutomationLogRedactor_ShouldMaskBearerTokensAndSecretAssignments()
    {
        var redacted = AutomationLogRedactor.Redact("Bearer abc.def token=secret api_key=value password=hunter2 secret=hidden");

        Assert.Equal("Bearer [redacted] token=[redacted] api_key=[redacted] password=[redacted] secret=[redacted]", redacted);
    }

    [Fact]
    public void AutomationLogBuffer_ShouldRedactAndTruncateLogs()
    {
        var buffer = new AutomationLogBuffer(maxBytes: 32);

        buffer.Append("token=secret");
        buffer.Append("this line is too long for the buffer");
        buffer.Append("ignored after truncation");

        var logs = buffer.ToString();
        Assert.Contains("token=[redacted]", logs);
        Assert.Contains("[citadel] log output truncated at 32 bytes.", logs);
        Assert.DoesNotContain("secret", logs);
        Assert.DoesNotContain("ignored after truncation", logs);
    }

    [Fact]
    public void AutomationScriptBuilder_ShouldIncludeGeneratedClientCatalogAndRuntimeHelpers()
    {
        var run = new ActionRun(
            actionId: Guid.NewGuid(),
            actionName: "action-1",
            trigger: ActionRunTrigger.Manual,
            runAsActorId: Guid.NewGuid(),
            triggeredByActorId: Guid.NewGuid(),
            argsJson: """{"mode":"manual"}""",
            codeSnapshot: """const volumes = await citadel.volumes.listVolumes("platform-1", { Dangling: true });""",
            timeoutSeconds: 30);

        const string catalogJson = """
        [
          {"key":"listVolumes","method":"GET","path":"/api/v1/volumes/{platformId}","group":"volumes"},
          {"key":"createTag","method":"POST","path":"/api/v1/tags","group":"tags"},
          {"key":"listGitRepositories","method":"GET","path":"/api/v1/gitRepositories","group":"gitRepositories"}
        ]
        """;

        var script = AutomationScriptBuilder.Build("http://citadel.test/", "token-value", run, catalogJson);

        Assert.Contains("const __citadelBaseUrl = \"http://citadel.test\";", script);
        Assert.Contains("const __citadelToken = \"token-value\";", script);
        Assert.Contains("const __citadelEndpointCatalog =", script);
        Assert.Contains("\"key\":\"listVolumes\"", script);
        Assert.Contains(@"endpoint.path.replace(/\{([^}:]+)(?::[^}]+)?\}/g", script);
        Assert.Contains("const query = endpoint.method === \"GET\" ? operationArgs[index++] : undefined;", script);
        Assert.Contains("const body = endpoint.method === \"GET\" ? undefined : operationArgs[index++];", script);
        Assert.Contains("repositories: __citadelGenerated.groups.gitRepositories", script);
        Assert.Contains(run.CodeSnapshot, script);
    }
}
