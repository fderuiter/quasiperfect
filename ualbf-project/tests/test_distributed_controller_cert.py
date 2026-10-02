import os
import subprocess
import time
import json
import socket
import pytest
import verify_cert


def get_free_port():
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(('127.0.0.1', 0))
    port = s.getsockname()[1]
    s.close()
    return port


@pytest.mark.skipif(
    os.environ.get("GITHUB_ACTIONS") == "true",
    reason="Skip cargo subprocess test under GHA fast-feedback python checks",
)
def test_distributed_controller_cert_signing(tmp_path):
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    env = dict(os.environ)
    if "LEAN_SYSROOT" not in env:
        env["LEAN_SYSROOT"] = "DUMMY"
    try:
        subprocess.run(
            ["cargo", "build", "--release", "--features", "signing"],
            cwd=os.path.join(repo_root, "rust-engine"),
            check=True,
            env=env,
        )
    except subprocess.CalledProcessError:
        pytest.skip("Cargo build failed (Lean toolchain is absent); skipping test_distributed_controller_cert_signing")
    engine_bin = os.path.join(repo_root, "target", "release", "ualbf_engine")
    if not os.path.exists(engine_bin):
        engine_bin = os.path.join(repo_root, "rust-engine", "target", "release", "ualbf_engine")

    port = get_free_port()
    addr = f"127.0.0.1:{port}"

    cert_path = os.path.join(repo_root, "certificate.json")
    formal_cert_path = os.path.join(repo_root, "formal_certificate.json")
    checkpoint_path = os.path.join(repo_root, "checkpoint.json")

    for p in [cert_path, formal_cert_path, checkpoint_path]:
        if os.path.exists(p):
            os.remove(p)

    common_args = [
        "--proof-manifest", "proof_manifest.json",
        "--sieve-limit", "200",
        "--target-min-log10", "37",
        "--target-max-log10", "43",
    ]

    controller_proc = subprocess.Popen(
        [engine_bin, "--mode", "controller", "--controller-addr", addr] + common_args,
        cwd=repo_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    time.sleep(0.3)

    worker_proc = subprocess.Popen(
        [engine_bin, "--mode", "worker", "--controller-addr", addr] + common_args,
        cwd=repo_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    out_worker, err_worker = worker_proc.communicate(timeout=180)
    out_ctrl, err_ctrl = controller_proc.communicate(timeout=180)

    assert controller_proc.returncode == 0, f"Controller failed: {err_ctrl}"
    assert worker_proc.returncode == 0, f"Worker failed: {err_worker}"

    assert os.path.exists(cert_path), "certificate.json was not created by controller"
    assert os.path.exists(formal_cert_path), "formal_certificate.json was not created by controller"

    with open(cert_path, "r", encoding="utf-8") as f:
        cert = json.load(f)

    assert "telemetry" in cert
    tel = cert["telemetry"]
    assert "explored_ranges" in tel
    assert isinstance(tel["explored_ranges"], list)
    assert len(tel["explored_ranges"]) > 0

    # Verify certificate path continuity
    verify_cert.verify_telemetry_paths([cert])

    # Clean up
    for p in [cert_path, formal_cert_path, checkpoint_path]:
        if os.path.exists(p):
            os.remove(p)
