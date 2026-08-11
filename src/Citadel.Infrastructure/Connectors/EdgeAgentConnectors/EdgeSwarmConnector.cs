using Citadel.Swarm.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Google.Protobuf;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.EdgeAgentConnectors;

internal sealed class EdgeSwarmConnector(IEdgeAgentCommandRouter commandRouter) : ISwarmConnector
{
    public async Task<Result<IReadOnlyList<SwarmNodeResult>>> ListNodesAsync(
        ListSwarmNodesCommand command,
        CancellationToken cancellationToken = default)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out var addressError))
            return Result.Failure<IReadOnlyList<SwarmNodeResult>>(addressError!);

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.SwarmNodeList,
            new ListSwarmNodesRequest
            {
                MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit),
                IncludeTaskCounts = command.IncludeTaskCounts
            }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? Result.Success<IReadOnlyList<SwarmNodeResult>>(
                ListSwarmNodesResponse.Parser.ParseFrom(response.Payload).Map())
            : Result.Failure<IReadOnlyList<SwarmNodeResult>>(
                EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.SwarmNodeList, response));
    }

    public async Task<Result<SwarmNodeResult>> InspectNodeAsync(
        InspectSwarmNodeCommand command,
        CancellationToken cancellationToken = default)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(command.PlatformAddress, out var platformId, out var addressError))
            return Result.Failure<SwarmNodeResult>(addressError!);

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            EdgeAgentCommandKind.SwarmNodeInspect,
            new InspectSwarmNodeRequest { NodeId = command.NodeId }.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);

        return response.IsSuccess && response.Payload is not null
            ? SwarmNodeMessage.Parser.ParseFrom(response.Payload).Map()
            : Result.Failure<SwarmNodeResult>(
                EdgeConnectorHelpers.CommandFailure(EdgeAgentCommandKind.SwarmNodeInspect, response));
    }

    public Task<Result> UpdateNodeAsync(
        UpdateSwarmNodeCommand command,
        CancellationToken cancellationToken = default) =>
        SendMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmNodeUpdate,
            new UpdateSwarmNodeRequest
            {
                NodeId = command.NodeId,
                VersionIndex = checked((ulong)command.VersionIndex),
                Availability = command.Availability,
                Labels = { command.Labels.ToDictionary() }
            }, cancellationToken);

    public Task<Result<IReadOnlyList<SwarmServiceResult>>> ListServicesAsync(ListSwarmServicesCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmServiceList, new ListSwarmServicesRequest { MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit) }, ListSwarmServicesResponse.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<SwarmServiceResult>> InspectServiceAsync(InspectSwarmServiceCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmServiceInspect, new InspectSwarmServiceRequest { ServiceId = command.ServiceId }, SwarmServiceMessage.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<ManagedSwarmServiceMutationResult>> CreateServiceAsync(CreateManagedSwarmServiceCommand command, CancellationToken cancellationToken = default) =>
        SendServiceMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmServiceCreate, SwarmServiceTransportMappers.Map(command), command.OperationId, cancellationToken);
    public Task<Result<ManagedSwarmServiceMutationResult>> UpdateServiceAsync(UpdateManagedSwarmServiceCommand command, CancellationToken cancellationToken = default) =>
        SendServiceMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmServiceUpdate, SwarmServiceTransportMappers.Map(command), command.OperationId, cancellationToken);
    public Task<Result> RestartServiceAsync(RestartSwarmServiceCommand command, CancellationToken cancellationToken = default) =>
        SendMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmServiceRestart,
            new RestartSwarmServiceRequest { ServiceId = command.ServiceId }, cancellationToken);
    public Task<Result> DeleteInventoryServiceAsync(DeleteSwarmInventoryServiceCommand command, CancellationToken cancellationToken = default) =>
        SendMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmServiceDelete,
            new DeleteManagedSwarmServiceRequest { ServiceId = command.ServiceId }, cancellationToken);
    public Task<Result> DeleteServiceAsync(DeleteManagedSwarmServiceCommand command, CancellationToken cancellationToken = default) =>
        SendMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmServiceDelete, SwarmServiceTransportMappers.Map(command), cancellationToken, command.OperationId);
    public Task<Result<SwarmLogsResult>> GetServiceLogsAsync(GetSwarmServiceLogsCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmServiceLogs, new SwarmLogsRequest { ResourceId = command.ServiceId, Tail = SwarmInventoryLimits.NormalizeLogLines(command.Tail) }, SwarmLogsResponse.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<IReadOnlyList<SwarmTaskResult>>> ListTasksAsync(ListSwarmTasksCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmTaskList, new ListSwarmTasksRequest { MaxItems = SwarmInventoryLimits.Normalize(command.Limit) }, ListSwarmTasksResponse.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<SwarmTaskResult>> InspectTaskAsync(InspectSwarmTaskCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmTaskInspect, new InspectSwarmTaskRequest { TaskId = command.TaskId }, SwarmTaskMessage.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<SwarmLogsResult>> GetTaskLogsAsync(GetSwarmTaskLogsCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmTaskLogs, new SwarmLogsRequest { ResourceId = command.TaskId, Tail = SwarmInventoryLimits.NormalizeLogLines(command.Tail) }, SwarmLogsResponse.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<IReadOnlyList<SwarmNetworkResult>>> ListNetworksAsync(ListSwarmNetworksCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmNetworkList, new ListSwarmNetworksRequest { MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit) }, ListSwarmNetworksResponse.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<SwarmNetworkResult>> InspectNetworkAsync(InspectSwarmNetworkCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmNetworkInspect, new InspectSwarmNetworkRequest { NetworkId = command.NetworkId }, SwarmNetworkMessage.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<IReadOnlyList<SwarmSecretResult>>> ListSecretsAsync(ListSwarmSecretsCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmSecretList, new ListSwarmSecretsRequest { MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit) }, ListSwarmSecretsResponse.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<SwarmSecretResult>> InspectSecretAsync(InspectSwarmSecretCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmSecretInspect, new InspectSwarmSecretRequest { SecretId = command.SecretId }, SwarmSecretMessage.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<SwarmResourceCreationResult>> CreateSecretAsync(CreateSwarmSecretCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmSecretCreate, new CreateSwarmSecretRequest
        {
            Name = command.Name,
            Data = ByteString.CopyFrom(command.Data),
            Labels = { command.Labels.ToDictionary() }
        }, SwarmResourceCreateResponse.Parser,
            static response => new SwarmResourceCreationResult(response.ResourceId), cancellationToken);
    public Task<Result> UpdateSecretLabelsAsync(UpdateSwarmSecretLabelsCommand command, CancellationToken cancellationToken = default) =>
        SendMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmSecretUpdate, new UpdateSwarmResourceLabelsRequest
        {
            ResourceId = command.SecretId,
            VersionIndex = checked((ulong)command.VersionIndex),
            Labels = { command.Labels.ToDictionary() }
        }, cancellationToken);
    public Task<Result> DeleteSecretAsync(DeleteSwarmSecretCommand command, CancellationToken cancellationToken = default) =>
        SendMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmSecretDelete,
            new DeleteSwarmSecretRequest { SecretId = command.SecretId }, cancellationToken);
    public Task<Result<IReadOnlyList<SwarmConfigResult>>> ListConfigsAsync(ListSwarmConfigsCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmConfigList, new ListSwarmConfigsRequest { MaxItems = SwarmInventoryLimits.NormalizeConnectorLimit(command.Limit) }, ListSwarmConfigsResponse.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<SwarmConfigResult>> InspectConfigAsync(InspectSwarmConfigCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmConfigInspect, new InspectSwarmConfigRequest { ConfigId = command.ConfigId }, SwarmConfigMessage.Parser, static value => value.Map(), cancellationToken);
    public Task<Result<byte[]>> GetConfigDataAsync(InspectSwarmConfigCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmConfigData, new InspectSwarmConfigRequest { ConfigId = command.ConfigId }, SwarmConfigDataResponse.Parser, static value => value.Data.ToByteArray(), cancellationToken);
    public Task<Result<SwarmResourceCreationResult>> CreateConfigAsync(CreateSwarmConfigCommand command, CancellationToken cancellationToken = default) =>
        SendAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmConfigCreate, new CreateSwarmConfigRequest
        {
            Name = command.Name,
            Data = ByteString.CopyFrom(command.Data),
            Labels = { command.Labels.ToDictionary() }
        }, SwarmResourceCreateResponse.Parser,
            static response => new SwarmResourceCreationResult(response.ResourceId), cancellationToken);
    public Task<Result> UpdateConfigLabelsAsync(UpdateSwarmConfigLabelsCommand command, CancellationToken cancellationToken = default) =>
        SendMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmConfigUpdate, new UpdateSwarmResourceLabelsRequest
        {
            ResourceId = command.ConfigId,
            VersionIndex = checked((ulong)command.VersionIndex),
            Labels = { command.Labels.ToDictionary() }
        }, cancellationToken);
    public Task<Result> DeleteConfigAsync(DeleteSwarmConfigCommand command, CancellationToken cancellationToken = default) =>
        SendMutationAsync(command.PlatformAddress, EdgeAgentCommandKind.SwarmConfigDelete,
            new DeleteSwarmConfigRequest { ConfigId = command.ConfigId }, cancellationToken);

    public Task<Result<ManagedSwarmServiceMutationResult>> CreateSystemServiceAsync(
        CreateSystemSwarmServiceCommand command,
        CancellationToken cancellationToken = default) =>
        SendServiceMutationAsync(
            command.PlatformAddress,
            EdgeAgentCommandKind.SwarmSystemServiceCreate,
            SwarmServiceTransportMappers.Map(command),
            command.OperationId,
            cancellationToken);

    public Task<Result<ManagedSwarmServiceMutationResult>> UpdateSystemServiceAsync(
        UpdateSystemSwarmServiceCommand command,
        CancellationToken cancellationToken = default) =>
        SendServiceMutationAsync(
            command.PlatformAddress,
            EdgeAgentCommandKind.SwarmSystemServiceUpdate,
            SwarmServiceTransportMappers.Map(command),
            command.OperationId,
            cancellationToken);

    private async Task<Result<TResult>> SendAsync<TMessage, TResult>(
        string platformAddress,
        EdgeAgentCommandKind kind,
        IMessage request,
        MessageParser<TMessage> parser,
        Func<TMessage, TResult> map,
        CancellationToken cancellationToken)
        where TMessage : IMessage<TMessage>
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(platformAddress, out var platformId, out var addressError))
            return Result.Failure<TResult>(addressError!);

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            kind,
            request.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: null,
            cancellationToken);
        return response.IsSuccess && response.Payload is not null
            ? Result.Success(map(parser.ParseFrom(response.Payload)))
            : Result.Failure<TResult>(EdgeConnectorHelpers.CommandFailure(kind, response));
    }

    private async Task<Result> SendMutationAsync(
        string platformAddress,
        EdgeAgentCommandKind kind,
        IMessage request,
        CancellationToken cancellationToken,
        Guid? operationId = null)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(platformAddress, out var platformId, out var addressError))
            return Result.Failure(addressError!);

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            kind,
            request.ToByteArray(),
            TimeSpan.FromSeconds(30),
            correlationId: operationId?.ToString("D"),
            cancellationToken);
        return response.IsSuccess
            ? Result.Success()
            : Result.Failure(EdgeConnectorHelpers.CommandFailure(kind, response));
    }

    private async Task<Result<ManagedSwarmServiceMutationResult>> SendServiceMutationAsync(
        string platformAddress,
        EdgeAgentCommandKind kind,
        IMessage request,
        Guid operationId,
        CancellationToken cancellationToken)
    {
        if (!EdgeConnectorHelpers.TryGetPlatformId(platformAddress, out var platformId, out var addressError))
            return Result.Failure<ManagedSwarmServiceMutationResult>(addressError!);

        var response = await commandRouter.SendUnaryAsync(
            platformId,
            kind,
            request.ToByteArray(),
            TimeSpan.FromSeconds(30),
            operationId.ToString("D"),
            cancellationToken);
        return response.IsSuccess && response.Payload is not null
            ? Result.Success(SwarmServiceTransportMappers.Map(SwarmServiceMutationResponse.Parser.ParseFrom(response.Payload)))
            : Result.Failure<ManagedSwarmServiceMutationResult>(EdgeConnectorHelpers.CommandFailure(kind, response));
    }
}
