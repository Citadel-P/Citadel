using System.Net.Security;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;
using Hosting.Common.Security;

namespace Tests.Unit.Infrastructure;

public sealed class CertificateTrustTests
{
    [Fact]
    public void Validate_AcceptsPlatformTrustedCertificateWhenCustomTrustIsConfigured()
    {
        using var certificates = TestCertificateChain.Create();
        using var trust = CreateTrust(certificates.Root);

        Assert.True(
            trust.Validate(
                this,
                certificates.Leaf,
                presentedChain: null,
                SslPolicyErrors.None));
    }

    [Fact]
    public void Validate_UsesIntermediatePresentedByServer()
    {
        using var certificates = TestCertificateChain.Create();
        using var trust = CreateTrust(certificates.Root);
        using var presentedChain = certificates.BuildPresentedChain();

        Assert.True(
            trust.Validate(
                this,
                certificates.Leaf,
                presentedChain,
                SslPolicyErrors.RemoteCertificateChainErrors));
    }

    [Fact]
    public void Validate_RejectsCertificateNameMismatch()
    {
        using var certificates = TestCertificateChain.Create();
        using var trust = CreateTrust(certificates.Root);
        using var presentedChain = certificates.BuildPresentedChain();

        Assert.False(
            trust.Validate(
                this,
                certificates.Leaf,
                presentedChain,
                SslPolicyErrors.RemoteCertificateNameMismatch));
    }

    private static CertificateTrust CreateTrust(X509Certificate2 root)
    {
        var path = Path.Combine(
            Path.GetTempPath(),
            $"citadel-agent-ca-{Guid.NewGuid():N}.pem");
        File.WriteAllText(path, root.ExportCertificatePem());
        try
        {
            return CertificateTrust.Load(path);
        }
        finally
        {
            File.Delete(path);
        }
    }

    private sealed class TestCertificateChain(
        X509Certificate2 root,
        X509Certificate2 intermediate,
        X509Certificate2 leaf)
        : IDisposable
    {
        public X509Certificate2 Root { get; } = root;
        public X509Certificate2 Intermediate { get; } = intermediate;
        public X509Certificate2 Leaf { get; } = leaf;

        public static TestCertificateChain Create()
        {
            using var rootKey = RSA.Create(2048);
            var rootRequest = CreateCertificateAuthorityRequest(
                "CN=Citadel Test Root",
                rootKey);
            using var rootWithKey = rootRequest.CreateSelfSigned(
                DateTimeOffset.UtcNow.AddMinutes(-5),
                DateTimeOffset.UtcNow.AddDays(7));
            var root = X509CertificateLoader.LoadCertificate(
                rootWithKey.RawData);

            using var intermediateKey = RSA.Create(2048);
            var intermediateRequest = CreateCertificateAuthorityRequest(
                "CN=Citadel Test Intermediate",
                intermediateKey);
            using var intermediateWithoutKey = intermediateRequest.Create(
                rootWithKey,
                DateTimeOffset.UtcNow.AddMinutes(-4),
                DateTimeOffset.UtcNow.AddDays(6),
                RandomNumberGenerator.GetBytes(16));
            var intermediate =
                intermediateWithoutKey.CopyWithPrivateKey(intermediateKey);

            using var leafKey = RSA.Create(2048);
            var leafRequest = new CertificateRequest(
                "CN=localhost",
                leafKey,
                HashAlgorithmName.SHA256,
                RSASignaturePadding.Pkcs1);
            var subjectAlternativeNames = new SubjectAlternativeNameBuilder();
            subjectAlternativeNames.AddDnsName("localhost");
            leafRequest.CertificateExtensions.Add(
                subjectAlternativeNames.Build());
            leafRequest.CertificateExtensions.Add(
                new X509BasicConstraintsExtension(
                    certificateAuthority: false,
                    hasPathLengthConstraint: false,
                    pathLengthConstraint: 0,
                    critical: true));
            leafRequest.CertificateExtensions.Add(
                new X509EnhancedKeyUsageExtension(
                    new OidCollection
                    {
                        new("1.3.6.1.5.5.7.3.1")
                    },
                    critical: false));
            var leaf = leafRequest.Create(
                intermediate,
                DateTimeOffset.UtcNow.AddMinutes(-3),
                DateTimeOffset.UtcNow.AddDays(5),
                RandomNumberGenerator.GetBytes(16));

            return new TestCertificateChain(root, intermediate, leaf);
        }

        public X509Chain BuildPresentedChain()
        {
            var chain = new X509Chain();
            chain.ChainPolicy.TrustMode = X509ChainTrustMode.CustomRootTrust;
            chain.ChainPolicy.RevocationMode = X509RevocationMode.NoCheck;
            chain.ChainPolicy.CustomTrustStore.Add(Root);
            chain.ChainPolicy.ExtraStore.Add(Intermediate);
            Assert.True(chain.Build(Leaf));
            return chain;
        }

        public void Dispose()
        {
            Leaf.Dispose();
            Intermediate.Dispose();
            Root.Dispose();
        }

        private static CertificateRequest CreateCertificateAuthorityRequest(
            string subject,
            RSA key)
        {
            var request = new CertificateRequest(
                subject,
                key,
                HashAlgorithmName.SHA256,
                RSASignaturePadding.Pkcs1);
            request.CertificateExtensions.Add(
                new X509BasicConstraintsExtension(
                    certificateAuthority: true,
                    hasPathLengthConstraint: false,
                    pathLengthConstraint: 0,
                    critical: true));
            request.CertificateExtensions.Add(
                new X509KeyUsageExtension(
                    X509KeyUsageFlags.KeyCertSign
                    | X509KeyUsageFlags.CrlSign,
                    critical: true));
            return request;
        }
    }
}
