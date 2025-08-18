using Application.Features.Containers.Commands;
using Domain;

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
    ContainerRestartPolicy RestartPolicy,
    Dictionary<string, string>? Labels,
    List<string>? EnvVars,
    List<string>? Ports,
    List<string>? Volumes,
    List<string>? Networks,
    List<string>? EntryPoint,
    List<string>? Command
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
