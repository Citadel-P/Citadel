using Application.Services;
using Domain.Contracts.Resources.Containers;

namespace Tests.Unit.Application.Services;

public sealed class ContainerInspectionRedactorTests
{
    [Fact]
    public void RedactEnvironment_ShouldMaskSensitiveValuesAndPreserveOrdinaryValues()
    {
        var inspection = Inspection(
            "API_TOKEN=do-not-return",
            "STRIPE_API_KEY=do-not-return",
            "POSTGRES_PASSWORD=do-not-return",
            "AWS_SECRET_ACCESS_KEY=do-not-return",
            "ConnectionStrings__Default=do-not-return",
            "SSH_PRIVATE_KEY=do-not-return",
            "PUBLIC_VALUE=visible",
            "APP_MODE=production",
            "MONKEY=banana",
            "TOKENIZER_MODEL=standard",
            "EMPTY=",
            "INHERITED");

        var result =
            ContainerInspectionRedactor.RedactEnvironment(inspection);

        Assert.Equal(
            [
                "API_TOKEN=********",
                "STRIPE_API_KEY=********",
                "POSTGRES_PASSWORD=********",
                "AWS_SECRET_ACCESS_KEY=********",
                "ConnectionStrings__Default=********",
                "SSH_PRIVATE_KEY=********",
                "PUBLIC_VALUE=visible",
                "APP_MODE=production",
                "MONKEY=banana",
                "TOKENIZER_MODEL=standard",
                "EMPTY=",
                "INHERITED"
            ],
            result.Config!.Env);
        Assert.Equal("API_TOKEN=do-not-return", inspection.Config!.Env[0]);
    }

    [Fact]
    public void RedactEnvironment_ShouldLeaveInspectionWithoutConfigurationUnchanged()
    {
        var inspection = Inspection() with { Config = null };
        var result =
            ContainerInspectionRedactor.RedactEnvironment(inspection);

        Assert.Same(inspection, result);
    }

    private static ContainerInspectionInfo Inspection(params string[] environment)
        => new(
            Id: "container-id",
            Created: "2026-07-26T00:00:00Z",
            Path: null,
            Args: [],
            State: null,
            Image: null,
            ResolvConfPath: null,
            HostnamePath: null,
            HostsPath: null,
            LogPath: null,
            Name: null,
            RestartCount: null,
            Driver: null,
            Platform: null,
            MountLabel: null,
            ProcessLabel: null,
            AppArmorProfile: null,
            ExecIDs: [],
            HostConfig: null,
            GraphDriver: null,
            SizeRw: null,
            SizeRootFs: null,
            Mounts: [],
            Config: new ContainerConfiguration(
                Hostname: null,
                Domainname: null,
                User: null,
                AttachStdin: null,
                AttachStdout: null,
                AttachStderr: null,
                ExposedPorts: null,
                Tty: null,
                OpenStdin: null,
                StdinOnce: null,
                Env: environment,
                Cmd: [],
                Image: null,
                Volumes: null,
                WorkingDir: null,
                Entrypoint: [],
                NetworkDisabled: null,
                MacAddress: null,
                OnBuild: [],
                Labels: new Dictionary<string, string>()),
            NetworkSettings: null);
}
