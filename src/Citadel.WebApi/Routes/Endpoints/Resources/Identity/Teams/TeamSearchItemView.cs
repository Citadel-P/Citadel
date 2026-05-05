using Domain.Contracts.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Teams;

public sealed record TeamSearchItemView(Guid Id, string Name)
{
    internal static TeamSearchItemView Map(TeamSearchItem item) => new(item.Id, item.Name);
}