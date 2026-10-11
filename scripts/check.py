import argparse
import hashlib
import os
from pathlib import Path
import platform
import shlex
import shutil
import subprocess
import sys
import tempfile
import zipfile


ROOT = Path(__file__).resolve().parent.parent
RESULTS = ROOT / "target" / "ci"
TEST_PROJECT = "windows/Diskvio.Windows.Tests/Diskvio.Windows.Tests.csproj"
ANALYZERS = [
    "-p:TreatWarningsAsErrors=true",
    "-p:EnforceCodeStyleInBuild=true",
    "-p:AnalysisLevel=latest-recommended",
]
SWIFTLINT_VERSION = "0.65.1"
SWIFTLINT_SHA256 = "c1e429b0599cf1b516f369a2d9ec04eaf0e436f3c12b637df8851fa52ff694d0"


def run(*command):
    print(f"\n> {shlex.join(str(argument) for argument in command)}", flush=True)
    executable = shutil.which(str(command[0]))
    if executable is None:
        raise RuntimeError(f"Required tool is missing from PATH: {command[0]}")
    subprocess.run([executable, *map(str, command[1:])], cwd=ROOT, check=True)


def rust():
    run("cargo", "fmt", "--all", "--", "--check")
    run("cargo", "check", "--workspace", "--all-targets", "--locked")
    run("cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings")
    run("cargo", "test", "--workspace", "--locked")


def swift():
    run("swift", "test", "--package-path", "macos", "-Xswiftc", "-warnings-as-errors")


def swiftlint():
    override = os.environ.get("DISKVIO_SWIFTLINT")
    if override:
        return override
    cache = Path(os.environ.get("DISKVIO_CHECK_TOOLS", ROOT / "target" / "check-tools"))
    destination = cache / f"swiftlint-{SWIFTLINT_VERSION}"
    executable = destination / "swiftlint"
    if not executable.is_file():
        destination.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="diskvio-swiftlint-") as temporary:
            archive = Path(temporary) / "swiftlint.zip"
            run(
                "curl", "--fail", "--location", "--retry", "3",
                f"https://github.com/realm/SwiftLint/releases/download/{SWIFTLINT_VERSION}/portable_swiftlint.zip",
                "-o", archive,
            )
            if hashlib.sha256(archive.read_bytes()).hexdigest() != SWIFTLINT_SHA256:
                raise RuntimeError("SwiftLint download checksum does not match the pinned release.")
            with zipfile.ZipFile(archive) as bundle:
                bundle.extract("LICENSE", destination)
                staged = Path(temporary) / "swiftlint"
                with bundle.open("swiftlint") as source, staged.open("wb") as output:
                    shutil.copyfileobj(source, output)
                staged.chmod(0o755)
                shutil.move(str(staged), executable)
    return executable


def macos(universal):
    run(swiftlint(), "lint", "--strict", "--no-cache", "--config", ".swiftlint.yml")
    swift()
    for configuration in ("Debug", "Release"):
        arguments = [
            "xcodebuild", "-project", "macos/Diskvio/Diskvio.xcodeproj",
            "-scheme", "Diskvio", "-configuration", configuration,
            "-destination", "platform=macOS" if configuration == "Debug" else "generic/platform=macOS",
            "-derivedDataPath", RESULTS / "DerivedData",
            "CODE_SIGNING_ALLOWED=NO", "SWIFT_TREAT_WARNINGS_AS_ERRORS=YES",
            "GCC_TREAT_WARNINGS_AS_ERRORS=YES",
        ]
        if configuration == "Debug":
            result = RESULTS / "Debug.xcresult"
            if result.exists():
                shutil.rmtree(result)
            arguments.extend(["test", "-only-testing:DiskvioTests", "-resultBundlePath", result])
        else:
            arguments.append("build")
            if universal:
                arguments.extend(["ARCHS=arm64 x86_64", "ONLY_ACTIVE_ARCH=NO"])
        run(*arguments)


def windows(native_integration):
    for configuration in ("Debug", "Release"):
        run(
            "msbuild", "windows/Diskvio.slnx", "-restore", "-m",
            f"-p:Configuration={configuration}", "-p:Platform=x64", *ANALYZERS,
            "-p:AppxPackageSigningEnabled=false", "-p:GenerateAppxPackageOnBuild=true",
            "-p:UapAppxPackageBuildMode=CI", "-p:AppxBundle=Never",
            "-p:EnableWinAppRunSupport=false",
        )
        arguments = [
            "dotnet", "test", TEST_PROJECT, "--no-build", "--no-restore",
            "--configuration", configuration, "-p:Platform=x64",
            "--logger", f"trx;LogFileName={configuration}.trx",
            "--results-directory", RESULTS / "windows",
        ]
        if not native_integration:
            arguments.extend(["--filter", "Category!=NativeIntegration"])
        run(*arguments)


def main():
    parser = argparse.ArgumentParser(description="Run Diskvio lint, build and test checks.")
    parser.add_argument("scope", nargs="?", choices=("all", "rust", "swift", "macos", "windows"), default="all")
    parser.add_argument("--universal", action="store_true")
    parser.add_argument("--native-integration", action="store_true")
    args = parser.parse_args()
    system = platform.system()
    if args.scope == "macos" and system != "Darwin":
        parser.error("macOS checks require a Mac with Xcode 27.")
    if args.scope == "windows" and system != "Windows":
        parser.error("Windows checks require Windows with Visual Studio 2026.")
    if args.scope in ("all", "rust"):
        rust()
    if args.scope == "swift":
        swift()
    if args.scope == "macos" or (args.scope == "all" and system == "Darwin"):
        macos(args.universal)
    if args.scope == "windows" or (args.scope == "all" and system == "Windows"):
        windows(args.native_integration)


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"\nChecks failed: {error}", file=sys.stderr)
        sys.exit(1)
