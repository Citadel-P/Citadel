using System.Runtime.CompilerServices;
using Citadel.Agent.Images.V1;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Domain.Entities;
using Domain.Entities.Registries;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Commands;

/// <summary>
/// Command to pull an image from a registry.
/// </summary>
public sealed record PullImage(Guid PlatformId, string RegistryName, string RepositoryName, string ImageTag) : IStreamCommand<PullImageResponse>
{
    internal class Validator : AbstractValidator<PullImage>
    {
        public Validator()
        {
            RuleFor(s => s.ImageTag).NotEmpty().NotNull();
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.RepositoryName).NotEmpty().NotNull();
            RuleFor(s => s.RegistryName).ValidNameIdentifier();
        }
    }
}

internal sealed class PullImageHandler(IGrpcClientFactory clientFactory, ApplicationDbContext dbContext) : IStreamCommandHandler<PullImage, PullImageResponse>
{
    public async IAsyncEnumerable<PullImageResponse> Handle(PullImage command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var platformAddress = await dbContext.Platforms
            .Where(s => s.Id == command.PlatformId)
            .Select(s => s.Address)
            .FirstOrDefaultAsync(cancellationToken)
            ?? throw new KeyNotFoundException("The provided platform Id does not exist");

        var registry = command.RegistryName == Registry.DefaultRegistryName
            ? Registry.DefaultRegistry() // Public Docker registry
            : await dbContext.Registries
                .AsNoTracking()
                .FirstOrDefaultAsync(s => s.Name == command.RegistryName, cancellationToken)
                ?? throw new KeyNotFoundException("The provided registry name does not exist");

        var client = clientFactory.GetImageClient(platformAddress);
        var request = CreatePullImageRequest(command, registry.Configuration);

        using var call = client.Pull(request, cancellationToken: cancellationToken);
        await foreach (var reply in call.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken).ConfigureAwait(false))
        {
            yield return reply;
        }
    }

    private static PullImageRequest CreatePullImageRequest(PullImage command, RegistryConfigurationBase registryCfg)
    {
        var request = new PullImageRequest();

        string domainName = registryCfg.RegistryUrl.Replace("https://", "");
        switch (registryCfg)
        {
            case GitHubRegistry ghCfg:
                request.FromImage = $"{domainName}/{ghCfg.Name}/{command.RepositoryName}@{command.ImageTag}".ToLower();
                request.Repo = $"{domainName}/{ghCfg.Name}/{command.RepositoryName}".ToLower();
                request.FromSrc = ghCfg.RegistryUrl;
                request.Auth = ghCfg.GetRegistryAuth();
                break;

            case DockerHubRegistry dockerCfg:
                if (command.RegistryName == Registry.DefaultRegistryName)
                {
                    request.FromImage = $"{domainName}/{command.ImageTag}:latest".ToLower();
                    request.Repo = domainName;
                    request.FromSrc = dockerCfg.RegistryUrl;
                }
                else
                {
                    request.FromImage = $"{domainName}/{dockerCfg.UserName}/{command.RepositoryName}:{command.ImageTag}".ToLower();
                    request.Repo = $"{domainName}/{dockerCfg.UserName}/{command.RepositoryName}".ToLower();
                    request.FromSrc = dockerCfg.RegistryUrl;
                    request.Auth = dockerCfg.GetRegistryAuth();
                }
                break;

            default:
                throw new NotSupportedException("Unsupported registry configuration");
        }

        return request;
    }
}