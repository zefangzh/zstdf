# zstdf

Rust tools for reading semiconductor Standard Test Data Format (STDF) files,
exporting Parquet data, validating records, and generating interactive HTML
dashboards. An optional Python extension exposes the conversion APIs as `_zstdf`.

Build instructions: [Windows](#build-on-windows), [RHEL 9](#build-on-red-hat-enterprise-linux-9),
and [macOS](#build-on-macos). The Linux/macOS commands below are setup guidance;
they have not been build-tested on this Windows development machine.

## Build on Windows

These instructions target **64-bit Windows 10/11**. Python is optional for the
command-line tools. Cursor or VS Code can be used as the editor; Microsoft C++
Build Tools are still required for compilation.

### 1. Install Git, Rust, and C++ Build Tools

Run in PowerShell:

```powershell
winget install --id Git.Git -e
winget install --id Rustlang.Rustup -e
```

Install [Microsoft C++ Build Tools](https://learn.microsoft.com/en-us/windows/dev-environment/rust/setup).
In Visual Studio Installer, select **Desktop development with C++** and include:

- MSVC x64/x86 C++ build tools.
- A Windows 10 or Windows 11 SDK.

You do not need to use Visual Studio as your editor. See the
[official Rust installation guide](https://rust-lang.org/tools/install/) for
the Rust installer and Windows prerequisites.

After installation, reopen **Developer PowerShell for Visual Studio** and verify:

```powershell
git --version
rustup default stable-x86_64-pc-windows-msvc
cargo --version
where.exe link
```

The linker path should point to the Microsoft MSVC tools. If no path is returned,
modify the Build Tools installation to add the C++ tools and Windows SDK, then
reopen Developer PowerShell.

### 2. Clone the Repository

If moving from another computer, ensure the changes you need have been committed
and pushed there first. Cloning only retrieves changes available on GitHub.

```powershell
git clone https://github.com/zefangzh/zstdf.git
cd zstdf
```

Run all following build commands from this folder, which contains the top-level
`Cargo.toml`. The first build needs internet access to download dependencies.

### 3. Test and Build the CLI

These commands exclude the optional Python binding from testing:

```powershell
cargo test --workspace --exclude stdf-py --locked
cargo build --release -p stdf-cli --locked
```

The executable is `target\release\zstdf-cli.exe`.

## Build on Red Hat Enterprise Linux 9

Use Bash on an x86_64 or aarch64 RHEL 9 machine with access to its BaseOS and
AppStream repositories. Install the native compiler and build utilities:

```bash
sudo dnf install -y git gcc gcc-c++ make pkgconf-pkg-config ca-certificates
```

Check that `curl` is available (`curl --version`). If it is missing, install
`curl-minimal` with `sudo dnf install -y curl-minimal`. See Red Hat's
[C/C++ development guide](https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html-single/developing_c_and_cpp_applications_in_rhel_9/developing_c_and_cpp_applications_in_rhel_9)
for compiler setup.

Install current stable Rust for your native architecture using
[rustup](https://rust-lang.org/tools/install/), as your regular user:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/zstdf-rustup-init.sh
sh /tmp/zstdf-rustup-init.sh --default-toolchain stable
. "$HOME/.cargo/env"
rustup default stable
cargo --version
gcc --version
```

Clone, test, and build from the repository root:

```bash
git clone https://github.com/zefangzh/zstdf.git
cd zstdf
cargo test --workspace --exclude stdf-py --locked
cargo build --release -p stdf-cli --locked
./target/release/zstdf-cli --help
```

Python is not needed for these CLI build/test commands. The executable is
`target/release/zstdf-cli`, without an `.exe` suffix. Do not use the Windows
`stable-x86_64-pc-windows-msvc` toolchain on Linux.

## Build on macOS

Use Terminal's native Zsh or Bash on your current macOS release. On Apple
silicon, use an arm64 terminal and matching Python installation; avoid mixing
Rosetta/x86_64 tools with native arm64 tools. On an Intel Mac, use x86_64 tools.

Install [Apple's Command Line Tools](https://developer.apple.com/documentation/xcode/installing-the-command-line-tools/):

```bash
xcode-select --install
```

Finish the installer before continuing. If the tools are already installed,
verify them and update them through Software Update when needed:

```bash
xcode-select -p
clang --version
git --version
uname -m
```

Install Rust with [rustup](https://rust-lang.org/tools/install/). It selects the
native macOS toolchain; the Windows MSVC toolchain is not needed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/zstdf-rustup-init.sh
sh /tmp/zstdf-rustup-init.sh --default-toolchain stable
. "$HOME/.cargo/env"
rustup default stable
cargo --version
```

Clone, test, and build:

```bash
git clone https://github.com/zefangzh/zstdf.git
cd zstdf
cargo test --workspace --exclude stdf-py --locked
cargo build --release -p stdf-cli --locked
./target/release/zstdf-cli --help
```

The executable is `target/release/zstdf-cli`. Homebrew and Python are optional
for the CLI; only the Python setup below uses Homebrew.

## Run the CLI

### Windows PowerShell

Replace the example paths below with files or directories that exist on your
computer. STDF input supports plain files and gzip compression.

Show available commands and inspect a file:

```powershell
.\target\release\zstdf-cli.exe --help
.\target\release\zstdf-cli.exe info C:\data\input.stdf
```

Convert to Parquet, then generate and open an interactive dashboard:

```powershell
.\target\release\zstdf-cli.exe convert C:\data\input.stdf output.parquet
.\target\release\zstdf-cli.exe dashboard output.parquet dashboard.html
Start-Process .\dashboard.html
```

The dashboard command requires an existing Parquet file. Run conversion
successfully before generating the dashboard.

For bounded, mixed-lot/wafer PTR conversion from a directory:

```powershell
.\target\release\zstdf-cli.exe convert-partitioned --output-dir dataset C:\data\inputs
```

This workflow writes multiple Parquet fragments and supports resource limits.
It currently supports PTR results; MPR/FTR expansion returns an explicit error.
Its memory budget applies to accounted conversion state, not an OS-enforced
resident-memory ceiling. The dashboard currently accepts one Parquet file at a
time. See [CLI documentation](docs/cli.md) for resource options, retries, and
dataset-generation handling.

Quote paths containing spaces, for example `"C:\test data\input.stdf"`.

### RHEL 9 and macOS

Use forward-slash paths and the native executable without `.exe`. Replace
`/path/to/input.stdf` and `/path/to/inputs` with actual paths:

```bash
./target/release/zstdf-cli info /path/to/input.stdf
./target/release/zstdf-cli convert /path/to/input.stdf output.parquet
./target/release/zstdf-cli dashboard output.parquet dashboard.html
./target/release/zstdf-cli convert-partitioned --output-dir dataset /path/to/inputs
```

Open the generated dashboard on macOS:

```bash
open dashboard.html
```

On a RHEL graphical desktop with `xdg-open` installed:

```bash
xdg-open dashboard.html
```

On a headless RHEL server, transfer `dashboard.html` to your desktop computer
and open it in a browser. The generated HTML is self-contained; no web server
is required. The same PTR-only and memory-accounting limits described above apply.

## Optional Python Binding

### Windows

Install **64-bit Python 3.12** with the Windows Python launcher (`py`). The Rust
and C++ build tools above are also required to build the extension from source.

From the repository root, create a virtual environment and install the package:

```powershell
py -3.12 -m venv .venv
$env:PYO3_PYTHON = (Resolve-Path .\.venv\Scripts\python.exe).Path
.\.venv\Scripts\python.exe -m pip install --upgrade pip
.\.venv\Scripts\python.exe -m pip install pyarrow .
.\.venv\Scripts\python.exe -c "import _zstdf; print(_zstdf.__version__)"
```

Using the virtual environment's Python directly avoids needing to activate it.
The installed distribution is named `stdf-rs`; its import name is `_zstdf`.

Example Python usage:

```python
import _zstdf

rows = _zstdf.write_parquet(r"C:\data\input.stdf", "output.parquet")
print(f"Converted {rows} rows")
```

To run the complete Rust workspace tests, including the Python binding, make
the base Python DLL directory available in the current shell:

```powershell
$env:PYO3_PYTHON = (Resolve-Path .\.venv\Scripts\python.exe).Path
$pythonBase = py -3.12 -c "import sys; print(sys.base_prefix)"
$env:Path = "$pythonBase;$env:Path"
cargo test --workspace --locked
```

### RHEL 9

Python 3.12 packages are available starting with **RHEL 9.4**. These optional
commands assume access to a RHEL 9 repository version providing those packages;
they are not required for the CLI on earlier RHEL 9 releases. See Red Hat's
[Python installation guide](https://developers.redhat.com/blog/install-python3-rhel).

```bash
sudo dnf install -y python3.12 python3.12-pip python3.12-devel
python3.12 -m venv .venv
export PYO3_PYTHON="$PWD/.venv/bin/python"
.venv/bin/python -m pip install --upgrade pip
.venv/bin/python -m pip install pyarrow .
.venv/bin/python -c "import _zstdf; print(_zstdf.__version__)"
```

Run these commands from the repository root. If the Python development package
is unavailable, ask your RHEL administrator to enable the appropriate approved
repositories; do not replace the OS-managed Python installation.

### macOS

If you do not already have Python 3.12, install [Homebrew](https://brew.sh/) and
follow its shell/PATH setup instructions. Then install its versioned
[Python 3.12 formula](https://formulae.brew.sh/formula/python%403.12):

```bash
brew install python@3.12
"$(brew --prefix python@3.12)/bin/python3.12" -m venv .venv
export PYO3_PYTHON="$PWD/.venv/bin/python"
.venv/bin/python -m pip install --upgrade pip
.venv/bin/python -m pip install pyarrow .
.venv/bin/python -c "import _zstdf; print(_zstdf.__version__)"
```

Run from the repository root. `brew --prefix` avoids hard-coding different
Homebrew locations on Apple silicon and Intel Macs. The virtual environment
keeps project dependencies separate from Homebrew's managed Python packages.

For Linux/macOS, verify the installed extension through the import smoke test
above and run the CLI-only Rust test command shown in the platform setup.
The binding currently enables PyO3's `extension-module` feature; full Rust
workspace test linking on Unix is not validated by these instructions.

Example conversion on either platform:

```bash
.venv/bin/python -c "import _zstdf; print(_zstdf.write_parquet('/path/to/input.stdf', 'output.parquet'))"
```

## Troubleshooting

| Error | What to check |
| --- | --- |
| `cargo` is not recognized | Reopen the shell after Rust installation. Verify `$HOME\.cargo\bin\cargo.exe` exists and `$HOME\.cargo\bin` is on PATH. |
| `link.exe` not found | Install the MSVC C++ tools and Windows SDK, then use Developer PowerShell. Cursor/VS Code alone does not provide the linker. |
| Could not find `Cargo.toml` | Change directory to the cloned repository root before running Cargo. |
| Python/PyO3 build errors | For CLI-only use, exclude `stdf-py` from workspace tests. For Python use, select the 64-bit Python 3.12 environment with `PYO3_PYTHON` as shown above. |
| File not found when generating a dashboard | Confirm conversion succeeded and supply the actual generated Parquet path. |
| Unknown `convert-partitioned` command | Ensure your checkout includes Phase 10C.1 and rebuild the release CLI. |
| `cargo: command not found` on Linux/macOS | Run `. "$HOME/.cargo/env"` or reopen your terminal after rustup installation. |
| `cc` or `clang` not found | On RHEL install GCC/build utilities; on macOS install the Command Line Tools and verify `xcode-select -p`. |
| RHEL cannot find Python 3.12 packages | Check that the enabled repositories provide RHEL 9.4 or later packages. Python is optional for the CLI. |
| Wrong architecture on macOS | Use matching native architectures for the terminal, Rust, Homebrew, and Python. Recreate the virtual environment if its Python architecture is wrong. |

## Project Documentation

- [CLI and Python usage](docs/cli.md)
- [Phased execution spec and implementation status](PHASED_EXECUTION_SPEC.md)
- [STDF project specification](ZSTDF_SPEC.md)
- [Implementation plan](implementation_plan.md)
- [Parquet profiling guide](docs/parquet_profiling.md)

Build artifacts under `target` are generated locally and do not need to be
copied from the original computer.
