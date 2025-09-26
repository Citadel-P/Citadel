using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities;

namespace Application.Services.SignalR;

internal interface IImageStreamManager : IStreamGroupManager
{
    Task SendImageInfo(Guid platformId, Image image);
    Task SendImagesInfo(Guid platformId, IEnumerable<Image> images);
}

internal class ImageStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IImageStreamManager
{
    public Task SendImageInfo(Guid platformId, Image image)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendImageInfo(platformId, image);
    }

    public Task SendImagesInfo(Guid platformId, IEnumerable<Image> images)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendImagesInfo(platformId, images);
    }
}

