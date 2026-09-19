#!/usr/bin/env bash
# Sets up a native Linux NyxAI build and a local-only AUTOMATIC1111 instance.
#
# This intentionally does not install GPU drivers, enable a system service,
# expose ports publicly, update an existing A1111 checkout, or download a
# checkpoint unless --model-url is explicitly supplied.

set -eEuo pipefail

readonly MIN_FREE_GIB=35
readonly A1111_REPOSITORY="https://github.com/AUTOMATIC1111/stable-diffusion-webui.git"

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
REPO_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
A1111_DIR="${A1111_DIR:-$HOME/AI/stable-diffusion-webui}"
A1111_PORT="${A1111_PORT:-7860}"
PYTHON_COMMAND="${PYTHON_COMMAND:-}"
MODEL_URL=""
MODEL_NAME=""
INSTALL_PREREQS=0
BUILD_NYXAI=1
SETUP_A1111=1
CHECK_GPU=1
ASSUME_YES=0

log() {
    printf '\n==> %s\n' "$*"
}

warn() {
    printf '\nWARNING: %s\n' "$*" >&2
}

die() {
    printf '\nERROR: %s\n' "$*" >&2
    exit 1
}

usage() {
    cat <<'EOF'
Usage: bash scripts/setup-linux.sh [options]

Set up NyxAI for native Linux use and install/reuse AUTOMATIC1111 Stable
Diffusion WebUI with its API enabled on loopback only.

Options:
  --install-prereqs        Install missing OS packages and Rustup after a prompt.
  --a1111-dir PATH        A1111 directory (default: ~/AI/stable-diffusion-webui).
  --a1111-port PORT       Local A1111 port (default: 7860).
  --python-command CMD    Compatible Python command for A1111 (for example python3.11).
  --model-url URL         Download one direct HTTPS .safetensors/.ckpt model URL.
  --model-name NAME       File name to use with --model-url (must end in .safetensors or .ckpt).
  --skip-nyxai-build      Do not run cargo build --release.
  --skip-a1111            Skip A1111 setup (build NyxAI only).
  --skip-gpu-check        Allow setup without a detected NVIDIA GPU/driver.
  --yes                   Do not ask before package installation.
  -h, --help              Show this help.

Safety defaults:
  * Existing A1111 installations and webui-user.sh files are never overwritten.
  * No model is downloaded unless --model-url is supplied explicitly.
  * A1111 gets --api and --port only. This script never adds --listen, --share,
    or a tunnel option, so the service remains local-only by default.
  * No GPU driver, firewall, systemd, Docker, or boot configuration is changed.
EOF
}

confirm() {
    local prompt="$1"
    if (( ASSUME_YES )); then
        return 0
    fi

    local answer
    read -r -p "$prompt [y/N] " answer || return 1
    [[ "$answer" =~ ^[Yy]([Ee][Ss])?$ ]]
}

nearest_existing_directory() {
    local path="$1"
    while [[ ! -d "$path" ]]; do
        local parent
        parent="$(dirname -- "$path")"
        [[ "$parent" != "$path" ]] || die "Could not find an existing parent directory for $1."
        path="$parent"
    done
    printf '%s\n' "$path"
}

available_bytes() {
    local path="$1"
    df -PB1 "$path" | awk 'NR == 2 { print $4 }'
}

format_gib() {
    awk -v bytes="$1" 'BEGIN { printf "%.1f GiB", bytes / 1024 / 1024 / 1024 }'
}

ensure_disk_space() {
    local target_parent free_bytes minimum_bytes
    target_parent="$(nearest_existing_directory "$A1111_DIR")"
    free_bytes="$(available_bytes "$target_parent")"
    minimum_bytes=$(( MIN_FREE_GIB * 1024 * 1024 * 1024 ))

    log "Disk-space check"
    printf 'Install filesystem: %s\nAvailable: %s\nRequired minimum: %s GiB\n' \
        "$target_parent" "$(format_gib "$free_bytes")" "$MIN_FREE_GIB"

    (( free_bytes >= minimum_bytes )) || die "Not enough free space for NyxAI, A1111 dependencies, and one typical checkpoint. Free at least ${MIN_FREE_GIB} GiB, then run the script again."
}

validate_port() {
    [[ "$A1111_PORT" =~ ^[0-9]+$ ]] || die "--a1111-port must be a valid TCP port."
    (( A1111_PORT >= 1 && A1111_PORT <= 65535 )) || die "--a1111-port must be between 1 and 65535."
}

check_linux() {
    [[ "$(uname -s)" == "Linux" ]] || die "This setup helper is for Linux."
    [[ "$(uname -m)" == "x86_64" || "$(uname -m)" == "amd64" ]] || die "This setup helper currently supports x86_64 Linux only."
}

check_nvidia() {
    (( CHECK_GPU )) || {
        warn "Skipping NVIDIA GPU verification at your request. A1111 may fall back to CPU if CUDA is not configured."
        return 0
    }

    command -v nvidia-smi >/dev/null 2>&1 || die "No NVIDIA driver utility was found. Install the appropriate NVIDIA driver for this Linux distribution, reboot manually if the driver installer requires it, verify 'nvidia-smi', then rerun this script."

    local gpu_info
    gpu_info="$(nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader 2>/dev/null)" \
        || die "nvidia-smi could not communicate with the NVIDIA driver. Fix the driver first; this script will not install or change drivers."

    log "NVIDIA GPU detected"
    printf '%s\n' "$gpu_info"
}

detect_package_manager() {
    if command -v apt-get >/dev/null 2>&1; then
        printf '%s\n' apt
    elif command -v dnf >/dev/null 2>&1; then
        printf '%s\n' dnf
    elif command -v pacman >/dev/null 2>&1; then
        printf '%s\n' pacman
    else
        die "Unsupported package manager. This helper supports apt, dnf, and pacman. Install Git, curl, wget, a compatible Python, Rust 1.82+, SQLite/OpenSSL build headers, and the official A1111 Linux dependencies manually, then rerun with --skip-a1111 or without --install-prereqs."
    fi
}

install_prerequisites() {
    (( INSTALL_PREREQS )) || return 0

    command -v sudo >/dev/null 2>&1 || die "--install-prereqs needs sudo to install system packages. Install the prerequisites manually or install sudo, then rerun."
    confirm "Install the required Linux packages with sudo?" || die "Prerequisite installation was declined. No changes were made."

    local manager
    manager="$(detect_package_manager)"
    log "Installing prerequisites with $manager"

    case "$manager" in
        apt)
            sudo apt-get update
            sudo apt-get install --yes --no-install-recommends \
                build-essential pkg-config libssl-dev libsqlite3-dev \
                ca-certificates curl git wget python3 python3-venv python3-dev \
                libgl1 libglib2.0-0
            ;;
        dnf)
            sudo dnf install --assumeyes \
                gcc gcc-c++ make pkgconf-pkg-config openssl-devel sqlite-devel \
                ca-certificates curl git wget python3 python3-devel \
                mesa-libGL glib2
            ;;
        pacman)
            sudo pacman -S --needed --noconfirm \
                base-devel pkgconf openssl sqlite curl git wget python python-pip \
                mesa glib2
            ;;
    esac
}

ensure_command() {
    local command_name="$1"
    local manual_hint="$2"
    command -v "$command_name" >/dev/null 2>&1 || die "$command_name is required. $manual_hint"
}

version_at_least() {
    local actual="$1" required="$2"
    [[ "$(printf '%s\n%s\n' "$required" "$actual" | sort -V | head -n1)" == "$required" ]]
}

ensure_rust() {
    if ! command -v cargo >/dev/null 2>&1; then
        (( INSTALL_PREREQS )) || die "Rust 1.82+ is required to build NyxAI. Install it with rustup or rerun with --install-prereqs to install rustup from https://sh.rustup.rs."
        ensure_command curl "Install curl, then rerun with --install-prereqs."
        confirm "Install Rust for this user with the official rustup installer?" || die "Rust installation was declined. No changes were made."
        log "Installing Rust through rustup"
        curl --proto '=https' --tlsv1.2 --fail --show-error --silent https://sh.rustup.rs | sh -s -- -y --profile minimal
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    fi

    local rust_version
    rust_version="$(rustc --version | awk '{ print $2 }')"
    version_at_least "$rust_version" "1.82.0" || die "NyxAI needs Rust 1.82+; found $rust_version. Update your existing Rust toolchain, then rerun."
    log "Rust toolchain"
    rustc --version
}

python_version_for() {
    "$1" -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")' 2>/dev/null
}

select_python() {
    local candidate version
    local candidates=()

    if [[ -n "$PYTHON_COMMAND" ]]; then
        candidates+=("$PYTHON_COMMAND")
    else
        candidates+=(python3.11 python3.10 python3)
    fi

    for candidate in "${candidates[@]}"; do
        command -v "$candidate" >/dev/null 2>&1 || continue
        version="$(python_version_for "$candidate")"
        if [[ "$version" == "3.10" || "$version" == "3.11" ]]; then
            PYTHON_COMMAND="$candidate"
            log "A1111 Python"
            printf '%s (%s)\n' "$PYTHON_COMMAND" "$version"
            return 0
        fi
    done

    die "AUTOMATIC1111's current Linux instructions call for Python 3.10 or 3.11. Install one through your distribution's supported package source, then rerun with --python-command python3.11 (or python3.10). This script does not add third-party package repositories automatically."
}

validate_model_name() {
    local name="$1"
    [[ "$name" != */* && "$name" != .* && "$name" != *..* ]] || die "Model file names must be a simple file name, not a path."
    [[ "$name" =~ \.(safetensors|ckpt)$ ]] || die "Model file name must end in .safetensors or .ckpt."
}

configure_a1111_user_file() {
    local user_file="$A1111_DIR/webui-user.sh"
    if [[ -e "$user_file" ]]; then
        if grep -Eq -- '(^|[^[:alnum:]_-])--api($|[^[:alnum:]_-])' "$user_file"; then
            log "Existing A1111 configuration preserved"
            printf 'API flag found in %s\n' "$user_file"
        else
            warn "Existing $user_file was preserved and does not appear to include --api. Add --api (and your preferred --port) to COMMANDLINE_ARGS before starting A1111, then set NyxAI to that private URL."
        fi
        return 0
    fi

    cat >"$user_file" <<EOF
#!/usr/bin/env bash
# Created by NyxAI's Linux setup helper. Kept local-only: do not add --listen,
# --share, or tunnel arguments unless you intentionally re-evaluate exposure.
python_cmd="$PYTHON_COMMAND"
export COMMANDLINE_ARGS="--api --port $A1111_PORT"
EOF
    chmod 700 "$user_file"
    log "Created A1111 user configuration"
    printf '%s\n' "$user_file"
}

setup_a1111() {
    (( SETUP_A1111 )) || return 0
    ensure_command git "Install Git or rerun with --install-prereqs."
    ensure_command wget "Install wget or rerun with --install-prereqs."
    if [[ -n "$MODEL_URL" ]]; then
        ensure_command curl "Install curl or rerun with --install-prereqs."
    fi
    select_python

    if [[ -e "$A1111_DIR" && ! -d "$A1111_DIR" ]]; then
        die "$A1111_DIR exists but is not a directory. Choose another path with --a1111-dir."
    fi

    if [[ -d "$A1111_DIR" ]]; then
        [[ -f "$A1111_DIR/webui.sh" ]] || die "$A1111_DIR already exists but does not look like an AUTOMATIC1111 checkout. It was not changed; choose another --a1111-dir."
        log "Reusing existing AUTOMATIC1111 checkout"
        printf '%s\n' "$A1111_DIR"
        warn "The existing checkout was not updated or overwritten. Update it yourself only when you are ready."
    else
        log "Cloning official AUTOMATIC1111 repository"
        mkdir -p "$(dirname -- "$A1111_DIR")"
        git clone --depth 1 "$A1111_REPOSITORY" "$A1111_DIR"
    fi

    configure_a1111_user_file

    local model_dir
    model_dir="$A1111_DIR/models/Stable-diffusion"
    mkdir -p "$model_dir"

    if [[ -n "$MODEL_URL" ]]; then
        [[ "$MODEL_URL" == https://* ]] || die "--model-url must use HTTPS."
        local resolved_model_name
        resolved_model_name="$MODEL_NAME"
        if [[ -z "$resolved_model_name" ]]; then
            resolved_model_name="$(basename -- "${MODEL_URL%%\?*}")"
        fi
        validate_model_name "$resolved_model_name"

        local remote_size free_bytes reserve_bytes
        remote_size="$(curl --fail --location --silent --show-error --head --write-out '%{content_length_download}' --output /dev/null "$MODEL_URL" 2>/dev/null || true)"
        free_bytes="$(available_bytes "$(nearest_existing_directory "$model_dir")")"
        reserve_bytes=$(( 5 * 1024 * 1024 * 1024 ))
        if [[ "$remote_size" =~ ^[0-9]+$ && "$remote_size" -gt 0 ]]; then
            printf 'Model download size: %s\n' "$(format_gib "$remote_size")"
            (( free_bytes > remote_size + reserve_bytes )) || die "There is not enough free disk space to download this model safely while retaining a 5 GiB reserve."
        else
            warn "The model host did not report a download size. The earlier ${MIN_FREE_GIB} GiB free-space check still applies."
        fi

        log "Downloading the explicitly requested checkpoint"
        curl --fail --location --continue-at - --output "$model_dir/$resolved_model_name" "$MODEL_URL"
    fi

    local models=() model
    while IFS= read -r -d '' model; do
        models+=("$model")
    done < <(find "$model_dir" -maxdepth 1 -type f \( -iname '*.safetensors' -o -iname '*.ckpt' \) -print0)

    log "A1111 checkpoint status"
    if (( ${#models[@]} == 0 )); then
        warn "No checkpoint is present in $model_dir. A1111 will install, but text-to-image cannot run until you place one compatible .safetensors/.ckpt checkpoint there or rerun with an explicit --model-url after accepting that model's license."
    else
        printf 'Found checkpoint(s):\n'
        printf '  %s\n' "${models[@]##*/}"
    fi

    printf 'A1111 revision: %s\n' "$(git -C "$A1111_DIR" rev-parse --short HEAD)"
}

build_nyxai() {
    (( BUILD_NYXAI )) || return 0
    ensure_command cargo "Install Rust or rerun with --install-prereqs."
    log "Building NyxAI release binary"
    (cd "$REPO_DIR" && cargo build --release)
}

print_next_steps() {
    local a1111_url="http://127.0.0.1:$A1111_PORT"
    cat <<EOF

Setup finished.

NyxAI repository: $REPO_DIR
NyxAI binary:     $REPO_DIR/target/release/nyxai
A1111 directory:  $A1111_DIR
A1111 API URL:    $a1111_url

Start A1111 (first start creates its isolated environment and downloads its
Python dependencies):
  cd "$A1111_DIR"
  ./webui.sh

With A1111 and Ollama running on this same Linux host, start NyxAI natively:
  cd "$REPO_DIR"
  A1111_BASE_URL="$a1111_url" OLLAMA_BASE_URL="http://127.0.0.1:11434" ./target/release/nyxai

Then open NyxAI Settings -> Image Generation, enable it, and use Test
connection / Refresh models. Verify A1111's API after it has started with:
  curl --fail --silent "$a1111_url/sdapi/v1/sd-models"

The default A1111 configuration intentionally binds locally. A NyxAI process
inside Docker cannot use its own localhost to reach this host-bound service.
For the private local-only arrangement above, run NyxAI natively. Do not add
--listen or --share merely to make a container reach A1111; use a deliberately
reviewed private networking configuration if you later choose Docker.
EOF
}

while (( $# > 0 )); do
    case "$1" in
        --install-prereqs) INSTALL_PREREQS=1 ;;
        --a1111-dir)
            [[ $# -ge 2 ]] || die "--a1111-dir needs a path."
            A1111_DIR="$2"
            shift
            ;;
        --a1111-port)
            [[ $# -ge 2 ]] || die "--a1111-port needs a port."
            A1111_PORT="$2"
            shift
            ;;
        --python-command)
            [[ $# -ge 2 ]] || die "--python-command needs a command."
            PYTHON_COMMAND="$2"
            shift
            ;;
        --model-url)
            [[ $# -ge 2 ]] || die "--model-url needs a direct HTTPS URL."
            MODEL_URL="$2"
            shift
            ;;
        --model-name)
            [[ $# -ge 2 ]] || die "--model-name needs a file name."
            MODEL_NAME="$2"
            shift
            ;;
        --skip-nyxai-build) BUILD_NYXAI=0 ;;
        --skip-a1111) SETUP_A1111=0 ;;
        --skip-gpu-check) CHECK_GPU=0 ;;
        --yes) ASSUME_YES=1 ;;
        -h|--help)
            usage
            exit 0
            ;;
        *) die "Unknown option: $1. Run with --help for usage." ;;
    esac
    shift
done

if [[ -n "$MODEL_NAME" && -z "$MODEL_URL" ]]; then
    die "--model-name can only be used together with --model-url."
fi

if (( ! SETUP_A1111 )) && [[ -n "$MODEL_URL" ]]; then
    die "--model-url cannot be used with --skip-a1111."
fi

check_linux
validate_port
if (( SETUP_A1111 )); then
    ensure_disk_space
    check_nvidia
fi
install_prerequisites

ensure_rust
setup_a1111
build_nyxai
print_next_steps
