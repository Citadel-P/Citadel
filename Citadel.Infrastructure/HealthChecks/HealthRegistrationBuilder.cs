namespace Infrastructure.HealthChecks;

using Contracts.Broker.EventMessaging;
using Microsoft.Extensions.DependencyInjection;

public static class HealthRegistrationBuilder
{
    public static IHealthChecksBuilder AddInfrastructureHealthCheck(this IHealthChecksBuilder builder)
    {
        builder.AddCheck<BrockerHealthCheck>("Event Bus");

        return builder;
    }
}