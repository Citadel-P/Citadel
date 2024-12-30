using LightResults;
using Hosting.Common.ErrorTypes;
using Microsoft.Extensions.Logging;

namespace Infrastructure.Services;

public interface IAgentService
{
    /// <summary>
    /// Query the agent to get the system info
    /// </summary>
    Task<Result<SystemInfoView>> GetSystemInfo(string platformAddress, CancellationToken cancellationToken = default);

    /// <summary>
    /// Get the list of containers
    /// </summary>
    Task<Result<IEnumerable<ContainerSummary>>> GetContainersList(string platformAddress, bool? all, int? limit, bool? size, IDictionary<string, IDictionary<string, bool>> filters, CancellationToken cancellationToken = default);

    /// <summary>
    /// Start a container(s)
    /// </summary>
    Task<Result> StartContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default);

    /// <summary>
    /// Stop a container(s)
    /// </summary>
    Task<Result> StopContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default);

    /// <summary>
    /// Pause a container(s)
    /// </summary>
    Task<Result> PauseContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default);

    /// <summary>
    /// Unpause a container(s)
    /// </summary>
    Task<Result> UnpauseContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default);

    /// <summary>
    /// Restart a container(s)
    /// </summary>
    Task<Result> RestartContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default);

    /// <summary>
    /// Delete a container(s)
    /// </summary>
    Task<Result> DeleteContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default);

    /// <summary>
    /// Request to start/stop streaming container logs
    /// </summary>
    Task<Result> StreamContainerLogs(string platformAddress, string containerId, Guid requestId, RequestedLogAction requestedLogAction, CancellationToken cancellation = default);
}

internal sealed class AgentService(IAgentProxy agentProxy, ILogger<AgentService> logger) : IAgentService
{
    /// <inheritdoc />
    public async Task<Result<SystemInfoView>> GetSystemInfo(string platformAddress, CancellationToken cancellationToken = default)
    {
        try
        {
            var result = await agentProxy.ForAddress(platformAddress).GetSystemInfo(cancellationToken);
            //result.AgentVersion = response.Headers.GetValues("agent-version").FirstOrDefault();
            return Result.Success(result);
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while contacting the remote agent");
            return Result.Failure<SystemInfoView>(new BadGatewayError(ex.Message));
        }
    }

    /// <inheritdoc />
    public async Task<Result<IEnumerable<ContainerSummary>>> GetContainersList(string platformAddress, bool? all, int? limit, bool? size, IDictionary<string, IDictionary<string, bool>> filters, CancellationToken cancellationToken = default)
    {
        try
        {
            var response = await agentProxy
                .ForAddress(platformAddress)
                .List(all, limit, size, filters, cancellationToken);

            return Result.Success(response.AsEnumerable());
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while contacting the remote agent");
            return Result.Failure<IEnumerable<ContainerSummary>>(new BadGatewayError(ex.Message));
        }
    }

    /// <inheritdoc />
    public async Task<Result> StartContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default)
    {
        try
        {
            await agentProxy.ForAddress(platformAddress).StartContainers(containersIds, cancellation);
            return Result.Success();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while contacting the remote agent");
            return Result.Failure(new BadGatewayError(ex.Message));
        }
    }

    /// <inheritdoc />
    public async Task<Result> StopContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default)
    {
        try
        {
            await agentProxy.ForAddress(platformAddress).StopContainers(containersIds, cancellation);
            return Result.Success();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while contacting the remote agent");
            return Result.Failure(new BadGatewayError(ex.Message));
        }
    }

    /// <inheritdoc />
    public async Task<Result> PauseContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default)
    {
        try
        {
            await agentProxy.ForAddress(platformAddress).PauseContainers(containersIds, cancellation);
            return Result.Success();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while contacting the remote agent");
            return Result.Failure(new BadGatewayError(ex.Message));
        }
    }

    /// <inheritdoc />
    public async Task<Result> UnpauseContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default)
    {
        try
        {
            await agentProxy.ForAddress(platformAddress).UnpauseContainers(containersIds, cancellation);
            return Result.Success();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while contacting the remote agent");
            return Result.Failure(new BadGatewayError(ex.Message));
        }
    }

    /// <inheritdoc />
    public async Task<Result> RestartContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default)
    {
        try
        {
            await agentProxy.ForAddress(platformAddress).RestartContainers(containersIds, cancellation);
            return Result.Success();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while contacting the remote agent");
            return Result.Failure(new BadGatewayError(ex.Message));
        }
    }

    /// <inheritdoc />
    public async Task<Result> DeleteContainers(string platformAddress, string[] containersIds, CancellationToken cancellation = default)
    {
        try
        {
            await agentProxy.ForAddress(platformAddress).DeleteContainers(containersIds, cancellation);
            return Result.Success();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while contacting the remote agent");
            return Result.Failure(new BadGatewayError(ex.Message));
        }
    }

    /// <inheritdoc />
    public async Task<Result> StreamContainerLogs(string platformAddress, string containerId, Guid requestId, RequestedLogAction requestedLogAction, CancellationToken cancellation = default)
    {
        try
        {
            var request = new StreamLogsRequest() 
            {
                RequestId = requestId,
                ContainerId = containerId, 
                RequestedLogAction = requestedLogAction
            };
            await agentProxy.ForAddress(platformAddress).StreamContainerLogs(request, cancellation);
            return Result.Success();
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while contacting the remote agent");
            return Result.Failure(new BadGatewayError(ex.Message));
        }
    }
}