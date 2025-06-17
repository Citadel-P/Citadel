using Application.Features.Auth.Models;
using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Auth.Commands;

public sealed record LoginCommand(string Email, string Password) : ICommand<Result<LoginResponse>>
{
    internal class Validator : AbstractValidator<LoginCommand>
    {
        public Validator()
        {
            RuleFor(x => x.Email).EmailAddress();
            RuleFor(x => x.Password).MinimumLength(8).MaximumLength(128);
        }
    }
}

internal sealed class LoginCommandHandler(IUnitOfWork unitOfWork, IJwtService jwtService) : ICommandHandler<LoginCommand, Result<LoginResponse>>
{
    public async ValueTask<Result<LoginResponse>> Handle(LoginCommand query, CancellationToken cancellationToken)
    {
        var user = await unitOfWork.Users.Query()
            .AsNoTracking()
            .Include(s => s.Teams)
            .ThenInclude(s => s.Role)
            .ThenInclude(s => s.Permissions)
            .FirstOrDefaultAsync(s => s.Email == query.Email, cancellationToken);

        if (user is null)
        {
            return Result.Failure<LoginResponse>(new NotFoundError("User does not exist"));
        }

        if (!user.IsValidPassword(query.Password))
        {
            return Result.Failure<LoginResponse>(new BadRequestError("Invalid credentials"));
        }

        var (accessToken, refreshToken) = await CreateTokens(user, cancellationToken);

        return Result.Success(new LoginResponse(accessToken));
    }

    private async Task<(string accessToken, string refreshToken)> CreateTokens(User user, CancellationToken cancellationToken)
    {
        var accessToken = jwtService.CreateAccessToken(user.GetJwtClaims());
        var (refreshTokenId, refreshToken) = jwtService.CreateRefreshToken();

        unitOfWork.RefreshTokens.Add(RefreshToken.Create(refreshTokenId, user.Id));

        // Limit the number of refresh tokens per user
        var tokensCount = await unitOfWork.RefreshTokens.Query().CountAsync(s => s.UserId == user.Id, cancellationToken);
        var maxTokensPerUser = 10;
        if (tokensCount > maxTokensPerUser)
        {
            var stallTokens = await unitOfWork.RefreshTokens.Query().AsNoTracking()
                .Where(s => s.UserId == user.Id)
                .OrderBy(s => s.CreatedAt).Take(tokensCount - maxTokensPerUser)
                .ToListAsync(cancellationToken);

            unitOfWork.RefreshTokens.RemoveRange(stallTokens);
        }

        await unitOfWork.SaveChangesAsync(cancellationToken);
        return (accessToken, refreshToken);
    }

}