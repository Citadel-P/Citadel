using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Gcontainers;
using Grpc.Core;

namespace Infrastructure.Services.Grpc;

internal class ContainersService : gContainers.gContainersBase
{
    public override Task<ContainersInfoReply> ContainersInfo(ContainersInfoMessage request, ServerCallContext context)
    {
        return base.ContainersInfo(request, context);
    }
}
