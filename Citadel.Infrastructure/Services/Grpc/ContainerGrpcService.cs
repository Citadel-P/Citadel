using Gcontainers;
using Grpc.Core;
using Infrastructure.Services.Abstractions;

namespace Infrastructure.Services.Grpc;

internal class ContainerGrpcService(IContainerService containerService) : gContainers.gContainersBase
{
    public override async Task<ContainersInfoReply> ContainersInfo(ContainersInfoMessage request, ServerCallContext context)
    {
        await containerService.OnContainersInfoMessage(request, context.CancellationToken);
        return new ContainersInfoReply();
    }

    public override async Task<ContainerEventReply> ContainerEvent(ContainerEventMessage request, ServerCallContext context)
    {
        await containerService.OnContainerEvent(request, context.CancellationToken);
        return new ContainerEventReply();
    }

    public override async Task<ContainerLogReply> ContainerLogs(ContainerLogMessage request, ServerCallContext context)
    {
        await containerService.OnContainerLogsMessage(request, context.CancellationToken);
        return new ContainerLogReply();
    }
}
