using Tests.Integration;
using Xunit.v3;

[assembly: AssemblyFixture(typeof(PostgresTestFixture))]
[assembly: Parallelization(MaxThreads = 8)]
