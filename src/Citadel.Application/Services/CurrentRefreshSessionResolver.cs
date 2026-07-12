using Domain.Contracts.Interfaces;

namespace Application.Services;

public interface ICurrentRefreshSessionResolver
{
    Task<Guid?> ResolveAsync(Guid userId, IUnitOfWork unitOfWork, CancellationToken cancellationToken);
}

internal sealed class CurrentRefreshSessionResolver(
    IJwtService jwtService,
    IRefreshTokenCookieService refreshTokenCookieService) : ICurrentRefreshSessionResolver
{
    public async Task<Guid?> ResolveAsync(Guid userId, IUnitOfWork unitOfWork, CancellationToken cancellationToken)
    {
        var refreshToken = refreshTokenCookieService.GetCurrent();
        if (string.IsNullOrWhiteSpace(refreshToken))
            return null;

        if (!jwtService.TryValidate(refreshToken, out var tokenId))
            return null;

        return await unitOfWork.RefreshTokens.GetActiveTokenIdAsync(
            tokenId,
            userId,
            DateTime.UtcNow,
            cancellationToken);
    }
}
