using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using LightResults;

namespace Infrastructure.Connectors;

internal class LocalPlatformConnector : IPlatformConnector
{
    public Task<PlatformHealth> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public Task<Result<Platform>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public IAsyncEnumerable<PlatformStatsBatch> StreamStatsAsync(StreamPlatformStatsCommand command, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
