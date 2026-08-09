using Application.Services;
using Domain;
using Domain.Entities.Stacks;

namespace Tests.Unit.Application.Features.Stacks;

public sealed class SwarmStackComposeCompatibilityTests
{
    [Fact]
    public void AnalyzeSwarmCompatibility_SupportedCompose_IsCompatible()
    {
        const string compose = """
        services:
          api:
            image: nginx:1.27
            deploy:
              mode: replicated
              replicas: 2
              update_config:
                order: start-first
            networks:
              - frontend
            volumes:
              - data:/var/lib/app
        networks:
          frontend:
            driver: overlay
        volumes:
          data:
            external: true
        """;

        var report = StackComposeParser.AnalyzeSwarmCompatibility([compose]);

        Assert.True(report.IsCompatible);
        Assert.Empty(report.Issues);
    }

    [Fact]
    public void AnalyzeSwarmCompatibility_EmptyNamedVolumeDefinitions_AreCompatible()
    {
        const string compose = """
        services:
          beszel:
            image: henrygd/beszel:latest
            volumes:
              - beszel_data:/beszel_data
              - beszel_socket:/beszel_socket
          beszel-agent:
            image: henrygd/beszel-agent:latest
            volumes:
              - beszel_agent_data:/var/lib/beszel-agent
              - beszel_socket:/beszel_socket
        volumes:
          beszel_data:
          beszel_agent_data:
          beszel_socket:
        """;

        var report = StackComposeParser.AnalyzeSwarmCompatibility([compose]);

        Assert.True(report.IsCompatible);
        Assert.DoesNotContain(report.Issues, issue => issue.Code == "compose.invalid_structure");
    }

    [Fact]
    public void AnalyzeSwarmCompatibility_UnsupportedAndReservedFields_FailsClosed()
    {
        const string compose = """
        services:
          api:
            image: nginx:latest
            container_name: fixed-api
            labels:
              com.citadel.stack-id: forged-owner
        """;

        var report = StackComposeParser.AnalyzeSwarmCompatibility([compose]);

        Assert.False(report.IsCompatible);
        Assert.Contains(report.Issues, issue =>
            issue.Code == "compose.unsupported_key"
            && issue.FieldPath == "services.api.container_name"
            && issue.Severity == SwarmStackCompatibilitySeverity.Error);
        Assert.Contains(report.Issues, issue => issue.Code == "labels.reserved");
    }

    [Fact]
    public void AnalyzeSwarmCompatibility_BuildRequiresMatchingCitadelBinding()
    {
        const string compose = """
        services:
          api:
            build: ./api
        """;

        var withoutBinding = StackComposeParser.AnalyzeSwarmCompatibility([compose]);
        var withBinding = StackComposeParser.AnalyzeSwarmCompatibility(
            [compose],
            [new StackBuildImageBinding("api", Guid.CreateVersion7())]);

        Assert.False(withoutBinding.IsCompatible);
        Assert.Contains(withoutBinding.Issues, issue => issue.Code == "service.build_unsupported");
        Assert.True(withBinding.IsCompatible);
    }

    [Fact]
    public void AnalyzeSwarmCompatibility_PortabilityRisks_AreWarnings()
    {
        const string compose = """
        services:
          api:
            image: nginx
            volumes:
              - ./config:/etc/nginx:ro
            ports:
              - target: 80
                published: 8080
                mode: host
        volumes:
          local-data:
            driver: local
        """;

        var report = StackComposeParser.AnalyzeSwarmCompatibility([compose]);

        Assert.True(report.IsCompatible);
        Assert.All(report.Issues, issue => Assert.Equal(SwarmStackCompatibilitySeverity.Warning, issue.Severity));
        Assert.Contains(report.Issues, issue => issue.Code == "mount.bind_portability");
        Assert.Contains(report.Issues, issue => issue.Code == "port.host_mode_portability");
        Assert.Contains(report.Issues, issue => issue.Code == "volume.local_portability");
    }

    [Fact]
    public void AnalyzeSwarmCompatibility_MultipleFiles_CanSupplyImageInOverride()
    {
        const string baseCompose = """
        services:
          api:
            environment:
              LOG_LEVEL: info
        """;
        const string overrideCompose = """
        services:
          api:
            image: nginx:latest
        """;

        var report = StackComposeParser.AnalyzeSwarmCompatibility([baseCompose, overrideCompose]);

        Assert.True(report.IsCompatible);
    }

    [Fact]
    public void AnalyzeSwarmCompatibility_UnsupportedEnumValue_FailsClosed()
    {
        const string compose = """
        services:
          api:
            image: nginx
            deploy:
              mode: magic
        """;

        var report = StackComposeParser.AnalyzeSwarmCompatibility([compose]);

        Assert.False(report.IsCompatible);
        Assert.Contains(report.Issues, issue =>
            issue.Code == "compose.unsupported_value"
            && issue.FieldPath == "services.api.deploy.mode");
    }

    [Fact]
    public void AnalyzeSwarmCompatibility_InvalidNestedStructure_FailsClosed()
    {
        const string compose = """
        services:
          api:
            image: nginx
            deploy: replicated
            ports:
              target: 80
        networks:
          - frontend
        """;

        var report = StackComposeParser.AnalyzeSwarmCompatibility([compose]);

        Assert.False(report.IsCompatible);
        Assert.Contains(report.Issues, issue =>
            issue.Code == "compose.invalid_structure"
            && issue.FieldPath == "services.api.deploy");
        Assert.Contains(report.Issues, issue =>
            issue.Code == "compose.invalid_structure"
            && issue.FieldPath == "services.api.ports");
        Assert.Contains(report.Issues, issue =>
            issue.Code == "compose.invalid_structure"
            && issue.FieldPath == "networks");
    }

    [Fact]
    public void ParseSwarmExternalResources_UsesExplicitNamesAndComposeKeys()
    {
        const string compose = """
        services:
          api:
            image: nginx
        networks:
          frontend:
            external: true
            name: shared-frontend
        secrets:
          password:
            external: true
        configs:
          settings:
            external: false
        volumes:
          data:
            external: true
            name: shared-data
        """;

        var references = StackComposeParser.ParseSwarmExternalResources([compose]);

        Assert.Equal(["shared-frontend"], references.Networks);
        Assert.Equal(["password"], references.Secrets);
        Assert.Empty(references.Configs);
        Assert.Equal(["shared-data"], references.Volumes);
    }

    [Theory]
    [InlineData(17, 0, "compose.too_many_files")]
    [InlineData(1, 4194305, "compose.too_large")]
    public void GetSwarmInputLimitReport_RejectsUnboundedInput(int fileCount, long totalBytes, string expectedCode)
    {
        var report = StackComposeParser.GetSwarmInputLimitReport(fileCount, totalBytes);

        Assert.NotNull(report);
        Assert.False(report.IsCompatible);
        Assert.Contains(report.Issues, issue => issue.Code == expectedCode);
    }
}
