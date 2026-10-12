from pathlib import Path
import shutil
import subprocess
import sys


ROOT = Path(__file__).resolve().parent.parent
PROJECT = ROOT / "macos" / "Diskvio" / "Diskvio.xcodeproj"


def main():
    if sys.platform != "darwin":
        raise RuntimeError("The Xcode Swift editor setup requires macOS.")

    server = shutil.which("xcode-build-server")
    if server is None:
        for directory in (Path("/opt/homebrew/bin"), Path("/usr/local/bin")):
            candidate = directory / "xcode-build-server"
            if candidate.is_file():
                server = str(candidate)
                break
    if server is None:
        raise RuntimeError("Install the editor bridge first: brew install xcode-build-server")

    subprocess.run(
        [server, "config", "-project", str(PROJECT), "-scheme", "Diskvio"],
        cwd=ROOT,
        check=True,
    )
    print("Build Diskvio in Xcode, then run Swift: Restart LSP Server in VS Code.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"Swift editor setup failed: {error}", file=sys.stderr)
        sys.exit(1)
