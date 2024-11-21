using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using Hosting.Common.ErrorTypes;
using Application.Features.Auth.Models;
using Application.Services;
using Infrastructure.Entities.Identity;
using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Caching.Memory;
using Microsoft.Extensions.Logging;
using Infrastructure;
using Infrastructure.EntityFramework;

namespace Application.Features.Auth.Queries;

public sealed record LoginQuery(string Email, string Password) : IQuery<Result<LoginResponse>>
{
    internal class Validator : AbstractValidator<LoginQuery>
    {
        public Validator()
        {
            RuleFor(x => x.Email).EmailAddress();
            RuleFor(x => x.Password).MinimumLength(8).MaximumLength(128);
        }
    }
}

internal sealed class LoginQueryHandler(IJwtService jwtService,
    IMemoryCache memoryCache,
    ApplicationDbContext dbContext,
    ILogger<LoginQueryHandler> logger)
    : IQueryHandler<LoginQuery, Result<LoginResponse>>
{
    public async ValueTask<Result<LoginResponse>> Handle(LoginQuery query, CancellationToken cancellationToken)
    {
        // Get user
        User user = await dbContext.Users.AsNoTracking().FirstOrDefaultAsync(s => s.Email == query.Email, cancellationToken: cancellationToken);
        if (user is null)
        {
            return Result.Fail<LoginResponse>(new NotFoundError("User does not exist"));
        }

        // Check password is valid
        if (!user.IsValidPassword(query.Password))
        {
            return Result.Fail<LoginResponse>(new BadRequestError("Invalid credentials"));
        }

        HashSet<Claim> claims =
        [
            new Claim("userId", user.Id.ToString()),
            new Claim("userName", user.Name),
            new Claim(JwtRegisteredClaimNames.Email, user.Email),
            new Claim(JwtRegisteredClaimNames.Jti, Guid.CreateVersion7().ToString())
        ];

        // Generate the jwt token
        string jwt = jwtService.CreateToken(claims);
        logger.LogInformation("A new Jwt has been issued for user {UserId}", user.Id);

        // Get permissions
        AppPermission[] permissions = await GetPermissionsAsync(user.Id);

        return Result.Ok(new LoginResponse(jwt, permissions));
    }

    private async ValueTask<AppPermission[]> GetPermissionsAsync(Guid userId)
    {
        string cacheKey = $"user-{userId}-permissions";
        // Check if permissions exist in cache
        if (memoryCache.TryGetValue(cacheKey, out AppPermission[] permissionsArray))
        {
            return permissionsArray;
        }
        else
        {
            User user = await dbContext.Users.AsNoTracking()
                                    .Include(s => s.Teams)
                                    .ThenInclude(s => s.Role)
                                    .ThenInclude(s => s.Permissions)
                                    .FirstAsync(s => s.Id == userId);

            HashSet<AppPermission> permissions = [];
            foreach (var team in user.Teams)
            {
                foreach (var permission in team.Role.Permissions)
                {
                    permissions.Add(permission.PermissionCode);
                }
            }

            permissionsArray = [.. permissions];
            memoryCache.Set($"user-{userId}-permissions", permissionsArray, DateTimeOffset.UtcNow.AddHours(12));
            logger.LogInformation("Permissions for user Id='{UserId}' has been set in cache", user.Id);

            return permissionsArray;
        }
    }
}