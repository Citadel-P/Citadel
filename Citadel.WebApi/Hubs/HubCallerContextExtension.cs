using Microsoft.AspNetCore.SignalR;

namespace WebApi.Hubs;

internal static class HubCallerContextExtension
{
    internal static string GetUserId(this HubCallerContext context) =>
        context.UserIdentifier ?? throw new InvalidOperationException("UserIdentifier is null");
}