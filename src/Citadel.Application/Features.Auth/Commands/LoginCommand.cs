using Application.Features.Auth.Models;
using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Auth.Commands;

public sealed record LoginCommand(string Email, string Password) : ICommand<Result<LoginResponse>>
{
    internal class Validator : AbstractValidator<LoginCommand>
    {
        public Validator()
        {
            RuleFor(x => x.Email).NotNull().EmailAddress();
            RuleFor(x => x.Password).NotNull().MinimumLength(8).MaximumLength(128);
        }
    }
}

internal sealed class LoginCommandHandler(IUnitOfWork unitOfWork, IJwtService jwtService) : ICommandHandler<LoginCommand, Result<LoginResponse>>
{
    public async ValueTask<Result<LoginResponse>> Handle(LoginCommand query, CancellationToken cancellationToken)
    {
        var userAuthInfo = await unitOfWork.Users.GetUserAuthInfoByEmailAsync(query.Email, cancellationToken);
        if (userAuthInfo is null)
        {
            return Result.Failure<LoginResponse>(new NotFoundError("User does not exist"));
        }

        if (!User.IsValidPassword(query.Password, userAuthInfo.Password ?? string.Empty))
        {
            return Result.Failure<LoginResponse>(new BadRequestError("Invalid credentials"));
        }

        var (accessToken, _) = await CreateTokens(userAuthInfo, cancellationToken);

        return Result.Success(new LoginResponse(accessToken));
    }

    private async Task<(string accessToken, string refreshToken)> CreateTokens(UserAuthInfo userAuthInfo, CancellationToken cancellationToken)
    {
        var accessToken = jwtService.CreateAccessToken(User.GetJwtClaims(userAuthInfo));
        var (refreshTokenId, refreshToken) = jwtService.CreateRefreshToken();

        await unitOfWork.RefreshTokens.AddAsync(RefreshToken.Create(refreshTokenId, userAuthInfo.Id), cancellationToken);

        // Limit the number of refresh tokens per userAuthInfo
        var tokensCount = await unitOfWork.RefreshTokens.CountAsync(userAuthInfo.Id, cancellationToken);
        var maxTokensPerUser = 10;
        if (tokensCount > maxTokensPerUser)
        {
            await unitOfWork.RefreshTokens.DeleteOldestTokensAsync(userAuthInfo.Id, tokensCount - maxTokensPerUser, cancellationToken);
        }

        await unitOfWork.CommitAsync();
        return (accessToken, refreshToken);
    }

}