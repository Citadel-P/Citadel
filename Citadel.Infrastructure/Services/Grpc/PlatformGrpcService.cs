using Gplatform;
using Grpc.Core;
using Infrastructure.Services.Abstractions;

namespace Infrastructure.Services.Grpc;

internal class PlatformGrpcService(IPlatformService platformService) : gPlatform.gPlatformBase
{
    public override async Task<SystemInfoReply> SystemInfo(SystemInfoMessage request, ServerCallContext context)
    {
        await platformService.OnSystemInfoMessage(request, context.CancellationToken);
        return new SystemInfoReply();
    }
}
