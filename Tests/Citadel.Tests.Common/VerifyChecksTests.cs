using System.Runtime.CompilerServices;
using DiffEngine;

namespace Tests.Common;

public class VerifyChecksTests
{
    [Fact]
    public Task Initialize() => VerifyChecks.Run();
}

public static class VerifyModuleInitializer
{
    [ModuleInitializer]
    public static void Initialize()
    {
        // Configure the directory for snapshots
        UseProjectRelativeDirectory("Verify");

        // Configure the diff tool to use
        DiffRunner.Disabled = false;
        DiffTools.UseOrder(DiffTool.VisualStudio, DiffTool.VisualStudioCode, DiffTool.Rider);
    }
}
