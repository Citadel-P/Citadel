using System;
using System.Collections.Generic;
using Application.Permissions;
using Hosting.Common.Attributes;

namespace Application.Services.Identity;

internal interface IPermissionCache
{
    PermissionMetadata? Get(PermissionCacheKey key);
    void Set(PermissionCacheKey key, PermissionMetadata meta);
    IReadOnlyCollection<PermissionCacheKey> GetIndex(Guid userId);
    void Remove(PermissionCacheKey key);
    void InvalidateUser(Guid userId);
}
