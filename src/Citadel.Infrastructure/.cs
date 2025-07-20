using System.Runtime.CompilerServices;
using Dapper;

[assembly: InternalsVisibleTo("Citadel.Tests.Integration")]
[assembly: InternalsVisibleTo("Citadel.Tests.Unit")]
[assembly: InternalsVisibleTo("DynamicProxyGenAssembly2")]

[module: DapperAot]
