using Domain.Contracts.Interfaces;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Features.GitRepositories;

internal static class GitRepositoryUrlValidation
{
    internal static async ValueTask<Result> ValidateAsync(
        IUnitOfWork unitOfWork,
        Guid? gitAccountId,
        string url,
        CancellationToken cancellationToken)
    {
        if (gitAccountId is null)
            return Result.Success();

        var gitAccount = await unitOfWork.GitAccounts.GetAsync(gitAccountId.Value, cancellationToken);
        if (gitAccount is null)
            return Result.Failure(new NotFoundError("The provided git account does not exist"));

        var repositoryDomain = ExtractDomain(url);
        var expectedDomain = NormalizeDomain(gitAccount.Domain);
        if (!string.Equals(repositoryDomain, expectedDomain, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure(new BadRequestError(
                $"Repository URL domain '{repositoryDomain}' does not match linked GitAccount domain '{expectedDomain}'."));
        }

        return Result.Success();
    }

    private static string ExtractDomain(string url)
    {
        if (Uri.TryCreate(url, UriKind.Absolute, out var absoluteUri))
            return NormalizeDomain(absoluteUri.Host);

        var atIndex = url.IndexOf('@');
        if (atIndex >= 0)
        {
            var hostStart = atIndex + 1;
            var colonIndex = url.IndexOf(':', hostStart);
            var slashIndex = url.IndexOf('/', hostStart);
            var hostEnd = colonIndex switch
            {
                < 0 when slashIndex >= 0 => slashIndex,
                < 0 => url.Length,
                _ when slashIndex >= 0 && slashIndex < colonIndex => slashIndex,
                _ => colonIndex
            };

            if (hostEnd > hostStart)
                return NormalizeDomain(url[hostStart..hostEnd]);
        }

        if (Uri.TryCreate($"https://{url}", UriKind.Absolute, out var relativeUri))
            return NormalizeDomain(relativeUri.Host);

        throw new ArgumentException("URL must be a valid git repository URL.", nameof(url));
    }

    private static string NormalizeDomain(string domain)
    {
        var trimmed = domain.Trim().TrimEnd('/');
        if (Uri.TryCreate(trimmed, UriKind.Absolute, out var absoluteUri))
            return absoluteUri.Host;

        return trimmed;
    }
}
