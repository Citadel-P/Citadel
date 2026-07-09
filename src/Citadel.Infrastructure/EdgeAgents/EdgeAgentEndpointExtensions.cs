using Microsoft.AspNetCore.Builder;

namespace Infrastructure.EdgeAgents;

public static class EdgeAgentEndpointExtensions
{
    public static WebApplication MapEdgeAgentGrpcService(this WebApplication app)
    {
        app.MapGrpcService<EdgeAgentGrpcService>();
        return app;
    }
}
