using Domain.Contracts.Resources.Identity;

namespace WebApi.Routes.Endpoints.Resources.Identity.Users;

public sealed record UserSearchItemView(Guid Id, string Name, string Email)
{
    internal static UserSearchItemView Map(UserSearchItem item) => new(item.Id, item.Name, item.Email);
}