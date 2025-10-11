using Application.Features.Containers.Commands;
using Domain;
using Domain.Entities;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record CreateContainerInput(
    Guid PlatformId,
    string ImageId,
    string? Name,
    string? WorkingDir,
    string? User,
    float? MemoryLimit,
    float? CpuLimit,
    float? MemoryReservation,
    bool? AutoRemove,
    int? StopTimeout,
    ContainerRestartPolicy RestartPolicy,
    Dictionary<string, string>? Labels,
    List<string>? EnvVars,
    List<string>? Ports,
    List<string>? Volumes,
    List<string>? Networks,
    List<string>? EntryPoint,
    List<string>? Command,
    string? Hostname = null,
    List<string>? Dns = null,
    SecurityConfig? Security = null,
    LoggingConfig? LoggingConfig = null,
    HealthCheckConfig? HealthCheck = null
    )
{
    internal CreateContainer ToCommand()
    {
        return new CreateContainer(
            PlatformId: PlatformId,
            ImageId: ImageId,
            Name: Name,
            WorkingDir: WorkingDir,
            User: User,
            MemoryLimit: MemoryLimit,
            CpuLimit: CpuLimit,
            MemoryReservation: MemoryReservation,
            AutoRemove: AutoRemove,
            RestartPolicy: RestartPolicy,
            Labels: Labels,
            EnvVars: EnvVars,
            Ports: Ports,
            Volumes: Volumes,
            Networks: Networks,
            EntryPoint: EntryPoint,
            Command: Command
        );
    }
}
