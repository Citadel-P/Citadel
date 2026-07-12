using Application.Services;
using Application.TaskJobs;

namespace Tests.Integration.Application.TaskJobs;

internal sealed class TestPlatformHealthBroadCaster : MulticastChannel<PlatformHealth>, IPlatformHealthBroadCaster;
