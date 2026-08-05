using System.Text;
using Domain;
using Application.TaskJobs;
using Application.Features.Swarm.Commands;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Swarm.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmServices(Guid PlatformId) : IQuery<Result<IReadOnlyList<SwarmServiceProjection>>>;
[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmService(Guid PlatformId, string ResourceId) : IQuery<Result<SwarmServiceProjection>>;
[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmTasks(Guid PlatformId, int Limit = 50) : IQuery<Result<IReadOnlyList<SwarmTaskProjection>>>
{
    internal sealed class Validator : AbstractValidator<GetSwarmTasks>
    {
        public Validator()
        {
            RuleFor(query => query.PlatformId).NotEmpty();
            RuleFor(query => query.Limit).InclusiveBetween(1, 200);
        }
    }
}
[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmTask(Guid PlatformId, string ResourceId) : IQuery<Result<SwarmTaskProjection>>;
[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmNetworks(Guid PlatformId) : IQuery<Result<IReadOnlyList<SwarmNetworkProjection>>>;
[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmNetwork(Guid PlatformId, string ResourceId) : IQuery<Result<SwarmNetworkProjection>>;
[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmSecrets(Guid PlatformId) : IQuery<Result<IReadOnlyList<SwarmSecretProjection>>>;
[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmSecret(Guid PlatformId, string ResourceId) : IQuery<Result<SwarmSecretProjection>>;
[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmConfigs(Guid PlatformId) : IQuery<Result<IReadOnlyList<SwarmConfigProjection>>>;
[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record GetSwarmConfig(Guid PlatformId, string ResourceId) : IQuery<Result<SwarmConfigProjection>>;
[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Inspect)]
public sealed record GetSwarmConfigData(Guid PlatformId, string ResourceId) : IQuery<Result<string>>;

internal static class SwarmQuery
{
    public static async Task<IError?> ValidateAsync(IUnitOfWork unitOfWork, Guid platformId, CancellationToken cancellationToken) =>
        await GetSwarmNodesHandler.ValidatePlatformAsync(unitOfWork, platformId, cancellationToken);
    public static Result<T> Found<T>(T? value, string type) where T : class =>
        value ?? Result.Failure<T>(new NotFoundError($"Swarm {type} does not exist."));

    public static async Task<IError?> ValidateAndEnsureInitializedAsync(
        IUnitOfWork unitOfWork,
        ISwarmReconciliationCoordinator reconciliationCoordinator,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var error = await ValidateAsync(unitOfWork, platformId, cancellationToken);
        if (error is not null)
            return error;

        if ((await unitOfWork.Swarm.GetNodesAsync(platformId, cancellationToken)).Count != 0)
            return null;

        var refresh = await reconciliationCoordinator.EnsureInitializedAsync(platformId, cancellationToken);
        return refresh.IsFailure(out error) ? error : null;
    }
}

internal sealed class GetSwarmServicesHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmServices, Result<IReadOnlyList<SwarmServiceProjection>>>
{
    public async ValueTask<Result<IReadOnlyList<SwarmServiceProjection>>> Handle(GetSwarmServices query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? Result.Success(await unitOfWork.Swarm.GetServicesAsync(query.PlatformId, ct)) : Result.Failure<IReadOnlyList<SwarmServiceProjection>>(error);
    }
}
internal sealed class GetSwarmServiceHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmService, Result<SwarmServiceProjection>>
{
    public async ValueTask<Result<SwarmServiceProjection>> Handle(GetSwarmService query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? SwarmQuery.Found(await unitOfWork.Swarm.GetServiceAsync(query.PlatformId, query.ResourceId, ct), "service") : Result.Failure<SwarmServiceProjection>(error);
    }
}
internal sealed class GetSwarmTasksHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmTasks, Result<IReadOnlyList<SwarmTaskProjection>>>
{
    public async ValueTask<Result<IReadOnlyList<SwarmTaskProjection>>> Handle(GetSwarmTasks query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? Result.Success(await unitOfWork.Swarm.GetTasksAsync(query.PlatformId, query.Limit, ct)) : Result.Failure<IReadOnlyList<SwarmTaskProjection>>(error);
    }
}
internal sealed class GetSwarmTaskHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmTask, Result<SwarmTaskProjection>>
{
    public async ValueTask<Result<SwarmTaskProjection>> Handle(GetSwarmTask query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? SwarmQuery.Found(await unitOfWork.Swarm.GetTaskAsync(query.PlatformId, query.ResourceId, ct), "task") : Result.Failure<SwarmTaskProjection>(error);
    }
}
internal sealed class GetSwarmNetworksHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmNetworks, Result<IReadOnlyList<SwarmNetworkProjection>>>
{
    public async ValueTask<Result<IReadOnlyList<SwarmNetworkProjection>>> Handle(GetSwarmNetworks query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? Result.Success(await unitOfWork.Swarm.GetNetworksAsync(query.PlatformId, ct)) : Result.Failure<IReadOnlyList<SwarmNetworkProjection>>(error);
    }
}
internal sealed class GetSwarmNetworkHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmNetwork, Result<SwarmNetworkProjection>>
{
    public async ValueTask<Result<SwarmNetworkProjection>> Handle(GetSwarmNetwork query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? SwarmQuery.Found(await unitOfWork.Swarm.GetNetworkAsync(query.PlatformId, query.ResourceId, ct), "network") : Result.Failure<SwarmNetworkProjection>(error);
    }
}
internal sealed class GetSwarmSecretsHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmSecrets, Result<IReadOnlyList<SwarmSecretProjection>>>
{
    public async ValueTask<Result<IReadOnlyList<SwarmSecretProjection>>> Handle(GetSwarmSecrets query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? Result.Success(await unitOfWork.Swarm.GetSecretsAsync(query.PlatformId, ct)) : Result.Failure<IReadOnlyList<SwarmSecretProjection>>(error);
    }
}
internal sealed class GetSwarmSecretHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmSecret, Result<SwarmSecretProjection>>
{
    public async ValueTask<Result<SwarmSecretProjection>> Handle(GetSwarmSecret query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? SwarmQuery.Found(await unitOfWork.Swarm.GetSecretAsync(query.PlatformId, query.ResourceId, ct), "secret") : Result.Failure<SwarmSecretProjection>(error);
    }
}
internal sealed class GetSwarmConfigsHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmConfigs, Result<IReadOnlyList<SwarmConfigProjection>>>
{
    public async ValueTask<Result<IReadOnlyList<SwarmConfigProjection>>> Handle(GetSwarmConfigs query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? Result.Success(await unitOfWork.Swarm.GetConfigsAsync(query.PlatformId, ct)) : Result.Failure<IReadOnlyList<SwarmConfigProjection>>(error);
    }
}
internal sealed class GetSwarmConfigHandler(IUnitOfWork unitOfWork, ISwarmReconciliationCoordinator reconciliationCoordinator) : IQueryHandler<GetSwarmConfig, Result<SwarmConfigProjection>>
{
    public async ValueTask<Result<SwarmConfigProjection>> Handle(GetSwarmConfig query, CancellationToken ct)
    {
        var error = await SwarmQuery.ValidateAndEnsureInitializedAsync(unitOfWork, reconciliationCoordinator, query.PlatformId, ct);
        return error is null ? SwarmQuery.Found(await unitOfWork.Swarm.GetConfigAsync(query.PlatformId, query.ResourceId, ct), "config") : Result.Failure<SwarmConfigProjection>(error);
    }
}

internal sealed class GetSwarmConfigDataHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory)
    : IQueryHandler<GetSwarmConfigData, Result<string>>
{
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);

    public async ValueTask<Result<string>> Handle(GetSwarmConfigData query, CancellationToken cancellationToken)
    {
        var context = await SwarmMutationContext.LoadAsync(
            unitOfWork,
            connectorFactory,
            query.PlatformId,
            cancellationToken);
        if (!context.IsSuccess(out var value, out var error))
            return Result.Failure<string>(error!);

        if (await unitOfWork.Swarm.GetConfigAsync(
                query.PlatformId,
                query.ResourceId,
                cancellationToken) is null)
            return Result.Failure<string>(new NotFoundError("Swarm config does not exist."));

        var result = await value.Connector.GetConfigDataAsync(
            new InspectSwarmConfigCommand(value.Platform.Address, query.ResourceId),
            cancellationToken);
        if (!result.IsSuccess(out var data, out error))
            return Result.Failure<string>(error!);

        try
        {
            return Result.Success(StrictUtf8.GetString(data));
        }
        catch (DecoderFallbackException)
        {
            return Result.Failure<string>(new BadRequestError(
                "This Swarm config contains binary data and cannot be displayed in the text editor."));
        }
    }
}
