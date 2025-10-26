using WebApi.Routes.Endpoints.Resources.Registries;
using Domain.Contracts.Resources.Images;

namespace WebApi.Routes.Endpoints.Resources.Images;

public sealed record InspectImageView(
    string Id,
    string Name,
    string Tag,
    long Size,
    string Os,
    string Created,
    string Architecture,
    IEnumerable<string> Env,
    IEnumerable<string> Cmd,
    IEnumerable<string> RepoTags,
    IEnumerable<string> Volumes,
    IEnumerable<string> ExposedPorts,
    IEnumerable<HistoryImageResult> Layers,
    IDictionary<string, string> Labels,
    IEnumerable<ContainerImageResult> Containers,
    RegistryView? Registry
    )
{

    public static InspectImageView Map (InspectImageResult image)
    {
        var nameTag = image.RepoTags.FirstOrDefault()?.Split(':', 2);
        return new InspectImageView(
            Id: image.Id,
            Name: nameTag is not null && nameTag.Length == 2 ? nameTag[0] : string.Empty,
            Tag: nameTag is not null && nameTag.Length == 2 ? nameTag[1] : string.Empty,
            Size: image.Size,
            Os: image.Os,
            Created: image.Created,
            Architecture: image.Architecture,
            Env: image.Env,
            Cmd: image.Cmd,
            RepoTags: image.RepoTags,
            Volumes: image.Volumes,
            ExposedPorts: image.ExposedPorts,
            Layers: image.Layers,
            Labels: image.Labels,
            Containers: image.Containers,
            Registry: image.Registry is not null ? RegistryView.Map(image.Registry) : null
        );
    }
};
