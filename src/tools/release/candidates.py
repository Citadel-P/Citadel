#!/usr/bin/env python3
"""Check each candidate's embedded versions and emit acceptance evidence."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).parent))
import release


def check_image(image, metadata, arch, config_digest):
    actual = json.loads(release.version.run("docker", "run", "--rm", "--network=none", image, "version-json"))
    for field, embedded in [("displayVersion", "version"), ("informationalVersion", "informationalVersion")]:
        if actual.get(embedded) != metadata[field]:
            raise ValueError(f"{image}: wrong embedded {embedded}")
    info = json.loads(release.version.run("docker", "image", "inspect", image))[0]
    labels = info["Config"]["Labels"]
    expected = {"org.opencontainers.image.version": metadata["productVersion"],
                "org.opencontainers.image.revision": metadata["sourceRevision"],
                "com.citadel.informational-version": metadata["informationalVersion"]}
    if (any(labels.get(k) != v for k, v in expected.items()) or info["Architecture"] != arch
            or actual.get("protocolVersion") != release.POLICY["protocolVersion"]
            or info["Id"] != config_digest):
        raise ValueError(f"{image}: OCI metadata differs from resolved identity")
    return dict(identity=actual, configDigest=info["Id"], architecture=arch)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", required=True, type=Path)
    parser.add_argument("--arch", required=True, choices=release.POLICY["architectures"])
    args = parser.parse_args()
    directory, arch = args.directory, args.arch
    metadata = json.loads((directory / "version.json").read_text())
    compatibility = json.loads((directory / "compatibility.json").read_text())
    images = {component: f"citadel-{component}:candidate" for component in ["agent", "core"]}
    tested = {}
    baselines = {}
    before = {c: release.version.sha256(directory / f"{c}-{arch}.oci.tar") for c in images}
    for component, image in images.items():
        with release.oci_source(f"oci-archive:{directory / (component + '-' + arch + '.oci.tar')}") as source:
            release.version.run("skopeo", "copy", source, "docker-daemon:" + image)
        with tempfile.TemporaryDirectory() as temporary:
            layout = Path(temporary)
            release.unpack(directory / f"{component}-{arch}.oci.tar", layout)
            _, config_digest = release.platform_manifest(layout, arch)
        tested[component] = check_image(image, metadata, arch, config_digest)
    # Existing scripts exercise actual packaged binaries, Docker, runtime tools,
    # enrollment, reconnect, builds, volumes and Swarm Node Agent lifecycle.
    subprocess.run(["bash", "test/scripts/test-runtime-image.sh", images["core"]], check=True)
    subprocess.run(["bash", "test/scripts/test-agent-image.sh", images["agent"], metadata["informationalVersion"]], check=True)
    subprocess.run(["bash", "test/scripts/test-agent-compatibility.sh", images["agent"]], check=True)
    subprocess.run(["bash", "test/scripts/test-agent-acceptance.sh", images["core"], images["agent"]], check=True)
    for component in ["agent", "core"]:
        baseline = compatibility.get(component)
        if baseline:
            release.version.run("docker", "pull", baseline)
            info = json.loads(release.version.run("docker", "image", "inspect", baseline))[0]
            candidate = json.loads(release.version.run("docker", "image", "inspect", images[component]))[0]
            if info["Architecture"] != arch:
                raise ValueError("Baseline architecture differs from the assigned runner")
            if info["Id"] == candidate["Id"]:
                raise ValueError("Candidate cannot serve as its own compatibility baseline")
            identity = json.loads(release.version.run("docker", "run", "--rm", "--network=none", baseline, "version-json"))
            if identity.get("protocolVersion") != compatibility["protocolVersion"]:
                raise ValueError("Baseline protocol is not covered by this compatibility policy")
            baselines[component] = dict(reference=baseline, identity=identity,
                                        configDigest=info["Id"], architecture=arch)
            # Run the full acceptance suite in BOTH supported upgrade directions.
            core = baseline if component == "core" else images["core"]
            agent = baseline if component == "agent" else images["agent"]
            subprocess.run(["bash", "test/scripts/test-agent-acceptance.sh", core, agent], check=True)
    after = {c: release.version.sha256(directory / f"{c}-{arch}.oci.tar") for c in images}
    if after != before:
        raise ValueError("Candidate archive changed during acceptance")
    evidence = dict(schemaVersion=1, passed=True, metadata=metadata, compatibility=compatibility,
                    artifacts=after, architecture=arch, testedImages=tested, baselineImages=baselines,
                    checks=["independent-binary-and-oci-versions", "core-runtime", "agent-image", "agent-compatibility", "candidate-acceptance"],
                    mixedVersionChecks=[c for c in ["agent", "core"] if compatibility.get(c)])
    (directory / f"evidence-{arch}.json").write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
