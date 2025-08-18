using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

public sealed record CreateContainer(
    Guid PlatformId,
    string ImageId,
    string? Name,
    string? WorkingDir,
    string? User,
    float? MemoryLimit,
    float? CpuLimit,
    float? MemoryReservation,
    bool? AutoRemove,
    ContainerRestartPolicy? RestartPolicy,
    Dictionary<string, string>? Labels,
    List<string>? EnvVars,
    List<string>? Ports,
    List<string>? Volumes,
    List<string>? Networks,
    List<string>? EntryPoint,
    List<string>? Command
    ) : ICommand<Result<string>>
{
    internal class Validator : AbstractValidator<CreateContainer>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.ImageId).NotEmpty().NotNull();
        }
    }
}

internal class CreateContainerHandler(IUnitOfWork unitOfWork, IConnectorFactory<IContainerConnector> connectorFactory)
    : ICommandHandler<CreateContainer, Result<string>>
{
    public async ValueTask<Result<string>> Handle(CreateContainer command, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(command.PlatformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<string>(new NotFoundError("No platform found for the given IDs."));
        }
        if (platform.CpuCount < command.CpuLimit)
        {
            return Result.Failure<string>(new BadRequestError("CPU limit exceeds platform's capacity."));
        }
        if (platform.MemTotal < command.MemoryLimit)
        {
            return Result.Failure<string>(new BadRequestError("Memory limit exceeds platform's capacity."));
        }
        if (platform.MemTotal < command.MemoryReservation)
        {
            return Result.Failure<string>(new BadRequestError("Memory reservation exceeds platform's capacity."));
        }
        var cpuQuota = command.CpuLimit.HasValue ? (command.CpuLimit.Value * 100000) : 0;

        var args = new CreateContainerCommand(
            PlatformAddress: platform.Address,
            ImageId: command.ImageId,
            Name: command.Name,
            WorkingDir: command.WorkingDir,
            User: command.User,
            MemoryLimit: (long?)(command.MemoryLimit * 1024 * 1024),
            CpuQuota: (long?)cpuQuota,
            MemoryReservation: (long?)(command.MemoryReservation * 1024 * 1024),
            AutoRemove: command.AutoRemove,
            RestartPolicy: command.RestartPolicy,
            Labels: command.Labels,
            EnvVars: command.EnvVars,
            Ports: command.Ports,
            Volumes: command.Volumes,
            Networks: command.Networks,
            EntryPoint: command.EntryPoint,
            Command: command.Command
        );
        return await connectorFactory.GetConnector(platform.ConnectorType).CreateAsync(args, cancellationToken);
    }
}
