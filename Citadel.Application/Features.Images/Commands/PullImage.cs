using System.Runtime.CompilerServices;
using Agent.Server.Images;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Commands;

public sealed record PullImage(Guid PlatformId, string RegistryName, string RepositoryName, string ImageTag) : IStreamCommand<PullImageReply>
{
    internal class Validator : AbstractValidator<PullImage>
    {
        public Validator()
        {
            RuleFor(s => s.ImageTag).NotEmpty().NotNull();
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.RepositoryName).NotEmpty().NotNull();
            RuleFor(s => s.RegistryName).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class PullImageHandler(IGrpcClientFactory clientFactory, ApplicationDbContext dbContext) : IStreamCommandHandler<PullImage, PullImageReply>
{
    public async IAsyncEnumerable<PullImageReply> Handle(PullImage command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == command.RegistryName, cancellationToken) 
            ?? throw new Exception("The provided registry name does not exist");

        var platformAddress = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken) 
            ?? throw new Exception("The provided platform Id does not exist");

        var client = clientFactory.GetImageClient(platformAddress);
        var request = new PullImageMessage();
        if (registry.Configuration is GitHubRegistry ghCfg)
        {
            request.FromImage = $"ghcr.io/{ghCfg.Name}/{command.RepositoryName}@{command.ImageTag}".ToLower();
            request.Repo = $"ghcr.io/{ghCfg.Name}/{command.RepositoryName}".ToLower();
            request.FromSrc = ghCfg.RegistryUrl;
            request.Auth = ghCfg.GetRegistryAuth();

            
        }
        else if (registry.Configuration is DockerHubRegistry dockerCfg)
        {
            request.FromImage = $"docker.io/{dockerCfg.UserName}/{command.RepositoryName}:{command.ImageTag}".ToLower();
            request.Repo = $"docker.io/{dockerCfg.UserName}/{command.RepositoryName}".ToLower();
            request.FromSrc = dockerCfg.RegistryUrl;
            request.Auth = dockerCfg.GetRegistryAuth();
        }

        using var call = client.PullImage(request, cancellationToken: cancellationToken);
        await foreach (var reply in call.ResponseStream.ReadAllAsync(cancellationToken: cancellationToken))
        {
            yield return reply;
        }
    }
}


