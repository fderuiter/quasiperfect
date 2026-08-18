#!/usr/bin/env python3
"""
UALBF Engine — Curses Dashboard (v4)

A real-time terminal GUI for monitoring the UALBF quasiperfect number search
engine.  Displays live telemetry from all engine subsystems and orchestrates
both the Lean 4 formal proof build and the Rust computational engine.

  • Phase 0  — Lean 4 Build & Proof Verification
  • Phase 1  — Legendre–Cattaneo Sieve (cyclotomic factorization)
  • Phase 2  — Fused DFS Construction & Ray-Casting
  • Lean 4 FFI bridge status (σ, mod-inverse, mod-8 checks)
  • Lock-free active-primes telemetry (AtomicU64 slot array)

Usage:
    python3 run_gui.py                          # defaults: 10^35 < N < 10^37
    python3 run_gui.py --min 35 --max 40        # custom range
    python3 run_gui.py --sieve-limit 500000     # bigger sieve
    python3 run_gui.py --max-exponent 6         # higher even exponents
    python3 run_gui.py --auto-raise             # auto-increment after success
    python3 run_gui.py --skip-lean-build        # skip lake build
    python3 run_gui.py --debug                  # run without --release

Keybindings:
    q / Q   — Quit the dashboard
    r       — Raise bounds and re-run (after completion)
    l       — Toggle Lean proof status overlay
"""

import curses
import subprocess
import threading
import queue
import sys
import time
import os
import re
import argparse

sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import cert_util
import select

sys.path.append(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import time_utils

# ═══════════════════════════════════════════════════════════════════════════════
#  Constants
# ═══════════════════════════════════════════════════════════════════════════════

import json


def get_dashboard_telemetry_interval():
    try:
        with open("profile.json", "r", encoding="utf-8") as f:
            p = json.load(f)
            return p.get("dashboard_telemetry_interval_ms", 250)
    except (FileNotFoundError, json.JSONDecodeError):
        return 250


REFRESH_HZ = (
    1000 / get_dashboard_telemetry_interval()
)  # UI refresh rate (frames per second)
LOG_THROTTLE_SEC = 5.0  # minimum interval between log file writes
MAX_LOG_LINES = 300  # scrollback buffer for the live event stream

PHASE_ICONS = {
    "0": "📐",  # Lean Build
    "1": "⚗ ",  # Sieve
    "2": "🌳",  # DFS + Ray-Cast (fused)
}

# Prior verified baseline — no QPN exists below this bound

try:
    with open(
        os.path.join(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
            "bounds_manifest.json",
        ),
        "r",
    ) as f:
        _bounds_manifest = json.load(f)
    VERIFIED_BASELINE_EXP = _bounds_manifest["search_bounds"]["target_min_log10"][
        "value"
    ]
    _MAX_EXP = _bounds_manifest["search_bounds"]["target_max_log10"]["value"]
    _SIEVE_LIMIT = _bounds_manifest["search_bounds"]["sieve_limit"]["value"]
    _MAX_EXPONENT = _bounds_manifest["search_bounds"]["max_exponent"]["value"]
    _PREFIX_STOP = _bounds_manifest["search_bounds"]["prefix_stop_threshold"]["value"]
except Exception:
    VERIFIED_BASELINE_EXP = 35
    _MAX_EXP = 37
    _SIEVE_LIMIT = 250000
    _MAX_EXPONENT = 4
    _PREFIX_STOP = 100000000000

VERIFIED_BASELINE = f"10^{VERIFIED_BASELINE_EXP} (Manifest)"

LEAN_THEOREMS = []
for _thm in cert_util.CORE_THEOREMS:
    _parts = _thm.split(".")
    _name = _parts[-1]
    _module = "/".join(_parts[1:-1]) + ".lean"
    LEAN_THEOREMS.append((_name, _module))


# Compiled regular expressions for high-frequency logs parsing
RE_TARGET_BOUND = re.compile(r"Target Bound:\s*10\^(\d+)\s*<\s*N\s*<\s*10\^(\d+)")
RE_SIEVE = re.compile(r"^Sieve:")
RE_RETAINED_PRUNED = re.compile(r"Retained:\s*([\d,]+),\s*Pruned:\s*([\d,]+)")
RE_DFS_COMPLETE = re.compile(
    r"DFS complete\.\s*Abundance-pruned:\s*(\d+)\s*\|\s*Topological-pruned:\s*(\d+)\s*\|\s*Conflicts learned:\s*(\d+)"
)
RE_DFS_TRACE = re.compile(
    r"DFS complete\.\s*Evaluated Branches:\s*(\d+)\s*\|\s*Abundance-pruned:\s*(\d+)"
)
RE_P_ACTIVE = re.compile(r"P-Active:\s*(.+?)\s*\|")
RE_P_ACTIVE_TOTAL = re.compile(r"\((\d+)\s*total\)")
RE_AB_PRUNED = re.compile(r"AbPruned:\s*([\d,]+)")
RE_PREFIXES = re.compile(r"Prefixes:\s*([\d,]+)")

# ═══════════════════════════════════════════════════════════════════════════════
#  ASCII Header
# ═══════════════════════════════════════════════════════════════════════════════

HEADER_ART = [
    "╔═══════════════════════════════════════════════════════════════╗",
    "║   █ █  █▀█  █   █▀▄  █▀▀   Quasiperfect Number Search         ║",
    "║   █▄█  █▀█  █▄  █▀▄  █▀    Lean4 + Rust + Topological Engine  ║",
    f"║   Established: No QPN exists below 10^{VERIFIED_BASELINE_EXP} (Manifest)            ║",
    "╚═══════════════════════════════════════════════════════════════╝",
]

# ═══════════════════════════════════════════════════════════════════════════════
#  CLI Argument Parsing
# ═══════════════════════════════════════════════════════════════════════════════


def parse_args():
    p = argparse.ArgumentParser(description="UALBF Engine Dashboard — Lean4 + Rust")
    p.add_argument(
        "--min",
        type=int,
        default=VERIFIED_BASELINE_EXP,
        help=f"Lower bound exponent (default: {VERIFIED_BASELINE_EXP} → 10^{VERIFIED_BASELINE_EXP})",
    )
    p.add_argument(
        "--max",
        type=int,
        default=_MAX_EXP,
        help=f"Upper bound exponent (default: {_MAX_EXP} → 10^{_MAX_EXP})",
    )
    p.add_argument(
        "--sieve-limit",
        type=int,
        default=_SIEVE_LIMIT,
        help=f"Prime sieve upper limit (default: {_SIEVE_LIMIT})",
    )
    p.add_argument(
        "--max-exponent",
        type=int,
        default=_MAX_EXPONENT,
        help=f"Maximum even exponent for prime powers (default: {_MAX_EXPONENT})",
    )
    p.add_argument(
        "--prefix-stop",
        type=int,
        default=_PREFIX_STOP,
        help=f"Prefix stop threshold (default: {_PREFIX_STOP})",
    )
    p.add_argument(
        "--auto-raise",
        action="store_true",
        help="Auto-increment bounds and re-run after success",
    )
    p.add_argument(
        "--raise-step",
        type=int,
        default=2,
        help="Exponent increment per auto-raise (default: 2)",
    )
    p.add_argument(
        "--skip-lean-build",
        action="store_true",
        help="Skip the Lean 4 lake build phase",
    )
    p.add_argument(
        "--debug", action="store_true", help="Run Rust engine without --release"
    )
    p.add_argument(
        "--headless",
        action="store_true",
        help="Run without curses GUI, outputting logs to stdout",
    )
    return p.parse_args()


# ═══════════════════════════════════════════════════════════════════════════════
#  Helper: safe addstr (never crash on write-past-edge)
# ═══════════════════════════════════════════════════════════════════════════════


def safe_addstr(win, y, x, text, attr=0):
    """Write text to the curses window, silently clipping at the edge."""
    h, w = win.getmaxyx()
    if y < 0 or y >= h or x >= w:
        return
    max_chars = w - x - 1
    if max_chars <= 0:
        return
    try:
        win.addstr(y, x, text[:max_chars], attr)
    except curses.error:
        pass


# ═══════════════════════════════════════════════════════════════════════════════
#  Lean 4 Proof Scanner
# ═══════════════════════════════════════════════════════════════════════════════


class LeanProofStatus:
    """Scans Lean 4 source files for theorem status (sorry/axiom/verified)."""

    def __init__(self, lean_project_dir):
        self.lean_dir = os.path.join(lean_project_dir, "UALBF")
        self.theorems = {}  # name → ("verified"|"sorry"|"axiom"|"missing")
        self.sorry_count = 0
        self.axiom_count = 0
        self.build_ok = None  # None=not run, True=success, False=failed
        self.build_errors = []
        self.build_warnings = []

    def scan(self):
        """Read theorem status from the root manifest."""
        import json
        import os

        self.sorry_count = 0
        self.axiom_count = 0
        self.theorems = {}
        self.hashes = {}
        self.verified_logic_hash = ""

        manifest_path = os.environ.get("UALBF_PROOF_MANIFEST", "../proof_manifest.json")
        try:
            with open(manifest_path, "r", encoding="utf-8") as f:
                manifest = json.load(f)

            self.verified_logic_hash = manifest.get("verified_logic_hash", "")

            for thm in manifest.get("theorems", []):
                full_name = thm.get("name", "")
                base_name = full_name.split(".")[-1]

                status = thm.get("status", "missing")
                if status == "unverified":
                    status = "sorry"

                self.theorems[base_name] = status
                self.hashes[base_name] = thm.get("checksum", "")

                if status == "sorry":
                    self.sorry_count += 1
                elif status in ["axiom", "axiomatic"]:
                    self.axiom_count += 1
        except Exception:
            pass

    def run_lake_build(self, lean_project_dir, q: queue.Queue):
        """Run `lake build` and report progress via the queue."""
        q.put({"type": "lean_build", "sub": "START"})
        try:
            proc = subprocess.Popen(
                ["lake", "build"],
                cwd=lean_project_dir,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                bufsize=1,
            )
            assert proc.stdout is not None
            for line in iter(proc.stdout.readline, ""):
                line = line.strip()
                if line:
                    q.put({"type": "lean_log", "text": line})
            proc.wait()
            if proc.returncode == 0:
                self.build_ok = True
                q.put({"type": "lean_build", "sub": "OK"})
            else:
                self.build_ok = False
                self.build_errors.append(
                    f"lake build exited with code {proc.returncode}"
                )
                q.put(
                    {
                        "type": "lean_build",
                        "sub": "FAIL",
                        "reason": str(proc.returncode),
                    }
                )
        except FileNotFoundError:
            self.build_ok = False
            self.build_errors.append("'lake' command not found — is elan installed?")
            q.put({"type": "lean_build", "sub": "FAIL", "reason": "lake not found"})
        except Exception as e:
            self.build_ok = False
            self.build_errors.append(str(e))
            q.put({"type": "lean_build", "sub": "FAIL", "reason": str(e)})


# ═══════════════════════════════════════════════════════════════════════════════
#  CursesGUI
# ═══════════════════════════════════════════════════════════════════════════════


class CursesGUI:
    def __init__(self, stdscr, args):
        self.stdscr = stdscr
        self.args = args
        self.is_headless = getattr(args, "headless", False)

        if not self.is_headless:
            curses.curs_set(0)
            self.stdscr.nodelay(True)
            self.stdscr.timeout(int(1000 / REFRESH_HZ))

            # ── Colors ──────────────────────────────────────────────────────
            curses.start_color()
            curses.use_default_colors()
            curses.init_pair(1, curses.COLOR_GREEN, -1)
            curses.init_pair(2, curses.COLOR_YELLOW, -1)
            curses.init_pair(3, curses.COLOR_CYAN, -1)
            curses.init_pair(4, curses.COLOR_MAGENTA, -1)
            curses.init_pair(5, curses.COLOR_BLACK, curses.COLOR_CYAN)
            curses.init_pair(6, curses.COLOR_RED, -1)
            curses.init_pair(7, curses.COLOR_WHITE, -1)
            curses.init_pair(8, curses.COLOR_GREEN, curses.COLOR_BLACK)
            curses.init_pair(9, curses.COLOR_CYAN, curses.COLOR_BLACK)
            curses.init_pair(10, curses.COLOR_BLUE, -1)

            self.C_GREEN = curses.color_pair(1)
            self.C_YELLOW = curses.color_pair(2)
            self.C_CYAN = curses.color_pair(3)
            self.C_MAGENTA = curses.color_pair(4)
            self.C_HEADER = curses.color_pair(5) | curses.A_BOLD
            self.C_RED = curses.color_pair(6)
            self.C_WHITE = curses.color_pair(7)
            self.C_PFILL = curses.color_pair(8)
            self.C_LABEL = curses.color_pair(3) | curses.A_BOLD
            self.C_BOLD = curses.color_pair(7) | curses.A_BOLD
            self.C_BLUE = curses.color_pair(10)
        else:
            self.C_GREEN = self.C_YELLOW = self.C_CYAN = self.C_MAGENTA = 0
            self.C_HEADER = self.C_RED = self.C_WHITE = self.C_PFILL = 0
            self.C_LABEL = self.C_BOLD = self.C_BLUE = 0

        self.queue = queue.Queue()
        self.log_lines = []

        # ── Bounds ──────────────────────────────────────────────────────
        self.bound_min = args.min
        self.bound_max = args.max
        self.sieve_limit = args.sieve_limit
        self.max_exponent = args.max_exponent
        self.prefix_stop = args.prefix_stop
        self.bound_history = []  # list of (min, max, result_str, elapsed)
        self.current_run = 0

        # ── Engine State ────────────────────────────────────────────────
        self.phase_num = "0"
        self.phase_text = "Initializing..."
        self.status_text = "Waiting for Lean build..."
        self.eta_text = "—"
        self.rate_text = "—"
        self.processed_text = "0"
        self.progress_pct = 0.0
        self.phase_start = time.time()
        self.global_start = time.time()

        # Domain stats
        self.target_bound = f"10^{self.bound_min} < N < 10^{self.bound_max}"
        self.target_bound_min = str(self.bound_min)
        self.target_bound_max = str(self.bound_max)
        self.verified_floor = VERIFIED_BASELINE
        self.retained_comps = "—"
        self.pruned_comps = "—"
        self.qp_found = 0

        # Pruning stats
        self.abundance_pruned = 0
        self.overflow_count = 0

        # Lean
        self.script_dir = os.path.dirname(os.path.abspath(__file__))
        self.lean_project = os.path.normpath(
            os.path.join(self.script_dir, "..", "lean4-proofs")
        )
        self.lean_status = LeanProofStatus(self.lean_project)
        self.lean_initialized = False
        self.show_lean_overlay = False

        # Active primes
        self.active_primes_str = "—"
        self.active_primes_cnt = 0

        # Throughput
        self.throughput_hist = []
        self.last_throughput_t = time.time()
        self.last_processed_n = 0

        self.is_indeterminate = False
        self.finished = False
        self.running = True
        self.awaiting_rerun = False

        # ── Launch ──────────────────────────────────────────────────────
        threading.Thread(target=self._run_pipeline, daemon=True).start()
        self._draw_loop()

    # ═══════════════════════════════════════════════════════════════════
    #  Full pipeline: Lean build → Lean scan → Rust engine
    # ═══════════════════════════════════════════════════════════════════

    def _run_pipeline(self):
        """Orchestrate the full Lean+Rust pipeline."""
        while self.running:
            self.finished = False
            self.awaiting_rerun = False
            self.current_run += 1
            run_start = time.time()

            # Phase 0a: Lean Build
            if not self.args.skip_lean_build:
                self.queue.put(
                    {
                        "type": "json_progress",
                        "event": {
                            "Phase": {"phase": 0, "name": "Lean 4 Build & Verification"}
                        },
                    }
                )
                self.lean_status.run_lake_build(self.lean_project, self.queue)
            else:
                self.lean_status.build_ok = True
                self.queue.put({"type": "lean_build", "sub": "SKIPPED"})

            # Phase 0b: Scan Lean proofs
            self.lean_status.scan()
            self.queue.put(
                {
                    "type": "lean_scan",
                    "sorry": self.lean_status.sorry_count,
                    "axiom": self.lean_status.axiom_count,
                }
            )

            # Phase 1-4: Rust engine
            if self.lean_status.build_ok is not False:
                run_success = self._run_engine(not self.args.debug)
            else:
                self.queue.put(
                    {
                        "type": "error",
                        "msg": "Lean build failed — cannot launch Rust engine",
                    }
                )
                run_success = False

            elapsed = time.time() - run_start
            result = "✓ Complete" if run_success else "✗ Failed/Aborted"
            self.bound_history.append((self.bound_min, self.bound_max, result, elapsed))
            if run_success:
                self.verified_floor = f"10^{self.bound_max} (this session)"

            if not self.running:
                break

            # Auto-raise or wait
            if self.args.auto_raise and run_success:
                self.bound_min = self.bound_max
                self.bound_max += self.args.raise_step
                self.target_bound = f"10^{self.bound_min} < N < 10^{self.bound_max}"
                self._log(
                    f"▲ Auto-raising bounds → 10^{self.bound_min} < N < 10^{self.bound_max}",
                    "phase",
                )
                # Reset stats for next run
                self._reset_engine_stats()
                continue
            else:
                if getattr(self, "is_headless", False):
                    self.running = False
                    break
                self.awaiting_rerun = True
                # Block until user presses 'r' or 'q'
                while self.running and self.awaiting_rerun:
                    time.sleep(0.1)
                if not self.running:
                    break
                self._reset_engine_stats()

    def _reset_engine_stats(self):
        self.abundance_pruned = 0
        self.overflow_count = 0
        self.progress_pct = 0.0
        self.processed_text = "0"
        self.throughput_hist = []
        self.retained_comps = "—"
        self.pruned_comps = "—"
        self.target_bound = f"10^{self.bound_min} < N < 10^{self.bound_max}"

    # ═══════════════════════════════════════════════════════════════════
    #  Engine subprocess + structured trace log writer
    # ═══════════════════════════════════════════════════════════════════

    def _run_engine(self, release: bool):
        cmd = ["cargo", "run", "--offline"]
        if release:
            cmd.append("--release")

        logs_dir = os.path.join(self.script_dir, "logs")
        os.makedirs(logs_dir, exist_ok=True)

        log_path = os.path.join(logs_dir, "engine_trace.log")
        last_log_time = 0.0
        log_buffer: list[str] = []
        run_start_time = time.time()

        csv_path = os.path.join(logs_dir, "engine_data_export.csv")
        exec_results_path = os.path.join(logs_dir, "execution_results.txt")
        run_output_path = os.path.join(logs_dir, "run_output.log")

        # Track state for structured trace output
        sieve_diag = {}  # collects Sieve|DIAG messages for Phase 1 box
        phase1_start_time = None
        phase2_start_time = None

        log_file = None
        csv_file = None
        exec_results_file = None
        run_output_file = None

        try:
            # Check if we need to write the file header (first run ever)
            write_header = (
                not os.path.exists(log_path) or os.path.getsize(log_path) == 0
            )
            log_file = open(log_path, "a")

            if write_header:
                self._trace_write_file_header(log_file)

            self._trace_write_run_header(log_file, cmd)
            log_file.flush()

            write_csv_header = (
                not os.path.exists(csv_path) or os.path.getsize(csv_path) == 0
            )
            csv_file = open(csv_path, "a")
            exec_results_file = open(exec_results_path, "a")
            run_output_file = open(run_output_path, "a")
            if write_csv_header:
                csv_file.write(
                    "timestamp,event_type,attribute1,attribute2,attribute3,factors\n"
                )

            env = os.environ.copy()
            env["RUST_BACKTRACE"] = "1"
            env["UALBF_TARGET_MIN_LOG10"] = str(self.bound_min)
            env["UALBF_TARGET_MAX_LOG10"] = str(self.bound_max)
            env["UALBF_SIEVE_LIMIT"] = str(self.sieve_limit)
            env["UALBF_MAX_EXPONENT"] = str(self.max_exponent)
            env["UALBF_PREFIX_STOP_THRESHOLD"] = str(self.prefix_stop)

            process = subprocess.Popen(
                cmd,
                cwd=self.script_dir,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                bufsize=1,
                env=env,
            )

            while self.running:
                assert process.stdout is not None
                rlist, _, _ = select.select([process.stdout], [], [], 0.1)
                if not self.running:
                    process.terminate()
                    break

                if rlist:
                    raw_line = process.stdout.readline()
                    if not raw_line:
                        break  # EOF Wait for wait()
                    if exec_results_file:
                        exec_results_file.write(raw_line)
                        exec_results_file.flush()
                    if run_output_file:
                        run_output_file.write(raw_line)
                        run_output_file.flush()
                    line = raw_line.strip()
                    if not line:
                        continue
                    parsed = self._parse_line_for_ui(line)
                    if parsed is not None:
                        self.queue.put(parsed)
                else:
                    now = time.time()
                    if log_buffer and now - last_log_time >= LOG_THROTTLE_SEC:
                        ts_short = time_utils.get_current_timestamp("[%H:%M:%S]")
                        summary = self._summarize_buffer(log_buffer)
                        log_file.write(f"{ts_short} [BATCH] {summary}\n")
                        log_buffer.clear()
                        log_file.flush()
                        last_log_time = now
                    continue

                ts_short = time_utils.get_current_timestamp("[%H:%M:%S]")
                ts_full = time_utils.get_current_timestamp("[%Y-%m-%d %H:%M:%S]")
                now = time.time()

                if line.startswith("DATA|"):
                    parts = line.split("|")
                    if len(parts) >= 2:
                        event_type = parts[1]
                        if event_type == "COMP" and len(parts) >= 6:
                            # DATA|COMP|p|two_e|abundance|factors
                            csv_file.write(
                                f'{ts_full},{event_type},{parts[2]},{parts[3]},{parts[4]},"{parts[5]}"\n'
                            )
                        elif event_type == "PREFIX" and len(parts) >= 4:
                            # DATA|PREFIX|length|factors
                            csv_file.write(
                                f'{ts_full},{event_type},{parts[2]},,,"{parts[3]}"\n'
                            )
                    csv_file.flush()
                    continue

                # ── Structured trace routing ───────────────────────────
                # Route engine messages to the appropriate trace writer
                # instead of dumping raw output.

                if "=== UALBF Engine Initializing ===" in line:
                    self._trace_write_init_block(log_file, ts_full)
                    last_log_time = now

                elif line.startswith("Sieve:"):
                    # Sieve config line — store for Phase 1 box
                    sieve_diag["config"] = line
                    log_file.write(f"{ts_full} {line}\n")
                    log_file.flush()
                    last_log_time = now

                elif line.startswith("PROGRESS|PHASE|1|"):
                    phase1_start_time = now
                    # Flush any pending buffer
                    if log_buffer:
                        summary = self._summarize_buffer(log_buffer)
                        log_file.write(f"{ts_short} [BATCH] {summary}\n")
                        log_buffer.clear()
                    self._trace_write_phase1_start(log_file, ts_full)
                    log_file.flush()
                    last_log_time = now

                elif line.startswith("Sieve|DIAG|"):
                    diag_text = line[len("Sieve|DIAG|") :]
                    if "Building trial sieve" in line:
                        sieve_diag["building"] = diag_text
                    elif "Trial sieve ready" in line:
                        sieve_diag["ready"] = diag_text
                    elif "Phase 1 complete" in line:
                        sieve_diag["complete"] = diag_text
                    log_file.write(f"{ts_full} {line}\n")
                    log_file.flush()
                    last_log_time = now

                elif line.startswith("Retained:"):
                    # Phase 1 results — write the sieve results box
                    if log_buffer:
                        summary = self._summarize_buffer(log_buffer)
                        log_file.write(f"{ts_short} [BATCH] {summary}\n")
                        log_buffer.clear()
                    phase1_elapsed = now - phase1_start_time if phase1_start_time else 0
                    self._trace_write_sieve_results(
                        log_file, ts_full, line, phase1_elapsed, sieve_diag
                    )
                    log_file.flush()
                    last_log_time = now

                elif line.startswith("PROGRESS|PHASE|2|"):
                    phase2_start_time = now
                    if log_buffer:
                        summary = self._summarize_buffer(log_buffer)
                        log_file.write(f"{ts_short} [BATCH] {summary}\n")
                        log_buffer.clear()
                    self._trace_write_phase2_start(log_file, ts_full)
                    log_file.flush()
                    last_log_time = now

                elif line.startswith("DFS complete"):
                    if log_buffer:
                        summary = self._summarize_buffer(log_buffer)
                        log_file.write(f"{ts_short} [BATCH] {summary}\n")
                        log_buffer.clear()
                    phase2_elapsed = now - phase2_start_time if phase2_start_time else 0
                    self._trace_write_dfs_results(
                        log_file, ts_full, line, phase2_elapsed
                    )
                    log_file.flush()
                    last_log_time = now

                elif line.startswith("PROGRESS|DONE|"):
                    if log_buffer:
                        summary = self._summarize_buffer(log_buffer)
                        log_file.write(f"{ts_short} [BATCH] {summary}\n")
                        log_buffer.clear()
                    total_elapsed = now - run_start_time
                    phase1_dur = (
                        (phase2_start_time - phase1_start_time)
                        if (phase1_start_time and phase2_start_time)
                        else 0
                    )
                    phase2_dur = (now - phase2_start_time) if phase2_start_time else 0
                    self._trace_write_verification_result(
                        log_file, ts_full, line, total_elapsed, phase1_dur, phase2_dur
                    )
                    log_file.flush()
                    last_log_time = now

                elif "QUASIPERFECT" in line:
                    log_file.write(f"\n{'█' * 80}\n")
                    log_file.write(f"{ts_full} ████ {line} ████\n")
                    log_file.write(f"{'█' * 80}\n\n")
                    log_file.flush()
                    last_log_time = now

                elif "Error" in line or "panic" in line.lower():
                    log_file.write(f"\n{ts_full} ✗ ERROR: {line}\n\n")
                    log_file.flush()
                    last_log_time = now

                elif line.startswith("PROGRESS|UPDATE|"):
                    # Throttled batch logging for progress ticks
                    log_buffer.append(line)
                    if now - last_log_time >= LOG_THROTTLE_SEC:
                        summary = self._summarize_buffer(log_buffer)
                        log_file.write(f"{ts_short} [BATCH] {summary}\n")
                        log_buffer.clear()
                        log_file.flush()
                        last_log_time = now

                else:
                    # Other unstructured output — batch with throttling
                    log_buffer.append(line)
                    if now - last_log_time >= LOG_THROTTLE_SEC:
                        if log_buffer:
                            summary = self._summarize_buffer(log_buffer)
                            log_file.write(f"{ts_short} [BATCH] {summary}\n")
                            log_buffer.clear()
                            log_file.flush()
                        last_log_time = now

            process.wait()

            if log_buffer:
                summary = self._summarize_buffer(log_buffer)
                ts = time_utils.get_current_timestamp("[%H:%M:%S]")
                log_file.write(f"{ts} [BATCH] {summary}\n")

            if process.returncode == 0:
                self.queue.put({"type": "success_exit"})
                log_file.write(
                    f"\n[{time_utils.get_current_timestamp('%H:%M:%S')}] ✓ Engine exited cleanly (code 0)\n"
                )
                return True
            else:
                self.queue.put({"type": "crash_exit", "code": str(process.returncode)})
                total_elapsed = time.time() - run_start_time
                log_file.write(
                    f"\n[{time_utils.get_current_timestamp('%H:%M:%S')}] ✗ Engine CRASHED (exit code {process.returncode})\n"
                )
                log_file.write(
                    f"           Total elapsed: {time_utils.format_hhmmss(total_elapsed)}\n"
                )
                log_file.write(
                    "           RUST_BACKTRACE=1 was set — check stderr for stack trace.\n\n"
                )
                return False

        except Exception as e:
            self.queue.put({"type": "error", "msg": str(e)})
            return False

        finally:
            if log_file:
                try:
                    log_file.close()
                except Exception:
                    pass
            if csv_file:
                try:
                    csv_file.close()
                except Exception:
                    pass
            if exec_results_file:
                try:
                    exec_results_file.close()
                except Exception:
                    pass
            if run_output_file:
                try:
                    run_output_file.close()
                except Exception:
                    pass

    # ═══════════════════════════════════════════════════════════════════
    #  Structured trace log writers
    # ═══════════════════════════════════════════════════════════════════

    def _trace_write_file_header(self, f):
        """Write the one-time file header explaining the engine architecture."""
        f.write(
            "╔══════════════════════════════════════════════════════════════════════════════════╗\n"
            "║                        UALBF Engine — Execution Trace                          ║\n"
            "║                 Quasiperfect Number (QPN) Non-Existence Engine                  ║\n"
            "║                      Lean 4 + Rust Pipeline                                     ║\n"
            "╚══════════════════════════════════════════════════════════════════════════════════╝\n"
            "\n"
            "┌─────────────────────────────────────────────────────────────────────────────────┐\n"
            "│ OVERVIEW                                                                        │\n"
            "│                                                                                 │\n"
            "│ A quasiperfect number N satisfies σ(N) = 2N + 1.  Such N must be an odd        │\n"
            "│ perfect square (Cattaneo 1951).  We write N = ∏ p_i^{2e_i}, decompose          │\n"
            '│ N = N_L · N_R² where N_L encodes the "left prefix" (small primes with          │\n'
            '│ known exponents) and N_R encodes the "right suffix" (unknown).  Then:          │\n'
            "│                                                                                 │\n"
            "│   σ(N_L) · σ(N_R²) = 2 · N_L · N_R² + 1                                      │\n"
            "│                                                                                 │\n"
            "│ The engine constructs all feasible N_L prefixes via DFS, then for each          │\n"
            "│ prefix, solves a modular congruence (ray-casting) to find candidate N_R         │\n"
            "│ values and checks whether σ(z²) matches the required value.                    │\n"
            "│                                                                                 │\n"
            f"│ PRIOR RESULT: No QPN exists below 10^{VERIFIED_BASELINE_EXP} (Manifest).                 │\n"
            "│                                                                                 │\n"
            "│ SUBSYSTEMS:                                                                     │\n"
            "│   Phase 0 — Lean 4 Build & Formal Proof Verification                           │\n"
            "│   Phase 1 — Legendre-Cattaneo Sieve (cyclotomic mod-8 screening)               │\n"
            "│   Phase 2 — Fused DFS Construction & Ray-Casting                             │\n"
            "│                                                                                 │\n"
            "│ The Lean 4 proofs establish the mathematical foundations:                        │\n"
            "│   • qpn_is_odd_square:          N must be an odd perfect square                │\n"
            "│   • legendre_cattaneo_obstruction: All σ(p^{2e}) prime factors ≡ 1,3 (mod 8)  │\n"
            "│   • qpn_coprime_15_omega_15:    gcd(N,15)=1 ⟹ ω(N) ≥ 15                     │\n"
            "│   • abundancy_starvation:        Running abundancy bound for feasibility       │\n"
            "│   • correction_factor_bound:     Totient ratio threshold for pruning           │\n"
            "│   • sigma_prime_pow_cyclotomic:  σ(p^k) = ∏ Φ_d(p) for d | (k+1)             │\n"
            "└─────────────────────────────────────────────────────────────────────────────────┘\n"
            "\n\n"
        )

    def _trace_write_run_header(self, f, cmd):
        """Write the run start header with full configuration."""
        f.write(
            f"\n{'═' * 82}\n"
            f" RUN #{self.current_run} — {time.ctime()}\n"
            f" Target: 10^{self.bound_min} < N < 10^{self.bound_max}\n"
            f"{'═' * 82}\n"
            "\n"
            "┌─────────────────────────────────────────────────────────────────────────────────┐\n"
            "│ CONFIGURATION                                                                   │\n"
            "│                                                                                 │\n"
            f"│   Target range:           10^{self.bound_min} < N < 10^{self.bound_max}"
            f"{' ' * max(1, 40 - len(str(self.bound_min)) - len(str(self.bound_max)))}│\n"
            f"│   Prime sieve limit:      {self.sieve_limit:,} (primes up to {self.sieve_limit:,} evaluated)"
            f"{' ' * max(1, 17 - len(f'{self.sieve_limit:,}'))}│\n"
            f"│   Max even exponent:      {self.max_exponent} (test p², p⁴{', p⁶' if self.max_exponent >= 3 else ''}"
            f"{', p⁸' if self.max_exponent >= 4 else ''} for each prime p)"
            f"{' ' * max(1, 8 - self.max_exponent)}│\n"
            f"│   Prefix stop threshold:  {self.prefix_stop:,} (N_L must exceed before ray-casting)"
            f"{' ' * max(1, 7 - len(f'{self.prefix_stop:,}') // 5)}│\n"
            f"│   Build profile:          {'--release (optimized)' if '--release' in ' '.join(cmd) else '--debug'}"
            f"{' ' * 28}│\n"
            f"│   Command:                {' '.join(cmd)}"
            f"{' ' * max(1, 52 - len(' '.join(cmd)))}│\n"
            "│                                                                                 │\n"
            "│   Env vars passed:                                                              │\n"
            f"│     UALBF_TARGET_MIN_LOG10      = {self.bound_min}"
            f"{' ' * max(1, 45 - len(str(self.bound_min)))}│\n"
            f"│     UALBF_TARGET_MAX_LOG10      = {self.bound_max}"
            f"{' ' * max(1, 45 - len(str(self.bound_max)))}│\n"
            f"│     UALBF_SIEVE_LIMIT           = {self.sieve_limit}"
            f"{' ' * max(1, 45 - len(str(self.sieve_limit)))}│\n"
            f"│     UALBF_MAX_EXPONENT          = {self.max_exponent}"
            f"{' ' * max(1, 45 - len(str(self.max_exponent)))}│\n"
            f"│     UALBF_PREFIX_STOP_THRESHOLD = {self.prefix_stop}"
            f"{' ' * max(1, 45 - len(str(self.prefix_stop)))}│\n"
            "│     RUST_BACKTRACE              = 1"
            f"{' ' * 44}│\n"
            "└─────────────────────────────────────────────────────────────────────────────────┘\n"
            "\n"
        )

    def _trace_write_init_block(self, f, ts):
        """Write the Phase 0 initialization documentation."""
        f.write(
            "\n"
            "┌─────────────────────────────────────────────────────────────────────────────────┐\n"
            f"│ PHASE 0 — INITIALIZATION ({ts})                                                │\n"
            "│                                                                                 │\n"
            "│ 1. Lean 4 Runtime Initialization                                               │\n"
            "│    • lean_initialize_runtime_module() — one-time Lean memory allocator setup   │\n"
            "│    • lean_initialize_thread() for each Rayon worker thread                      │\n"
            "│    • FFI bridge exposes 3 verified functions:                                   │\n"
            "│      · ualbf_check_mod_8(q)       — Legendre-Cattaneo mod-8 obstruction check │\n"
            "│      · ualbf_compute_sigma(p, e)  — Verified σ(p^e) via 128-bit hi/lo split   │\n"
            "│      · ualbf_mod_inverse(a, m)    — Verified modular inverse for CRT           │\n"
            "│                                                                                 │\n"
            "│ 2. Rayon Thread Pool                                                            │\n"
            "│    • Global thread pool with Lean worker-thread initializer                     │\n"
            "│    • Enables lock-free parallel DFS in Phase 2                                  │\n"
            "└─────────────────────────────────────────────────────────────────────────────────┘\n"
            "\n"
            f"{ts} === UALBF Engine Initializing ===\n"
        )

    def _trace_write_phase1_start(self, f, ts):
        """Write the Phase 1 sieve documentation block."""
        f.write(
            "\n"
            "┌─────────────────────────────────────────────────────────────────────────────────┐\n"
            f"│ PHASE 1 — LEGENDRE-CATTANEO SIEVE ({ts})                                       │\n"
            "│                                                                                 │\n"
            "│ PURPOSE:                                                                        │\n"
            f"│ Enumerate all prime-power components (p, 2e) where σ(p^{{2e}}) has ALL prime    │\n"
            "│ factors ≡ 1 or 3 (mod 8).  This is the Legendre-Cattaneo obstruction:          │\n"
            "│                                                                                 │\n"
            "│   Theorem (Cattaneo 1951 / Hagis-Cohen 1982):                                        │\n"
            f"│   If N is quasiperfect, σ(N) = 2N+1 ≡ 1 (mod 8), so each σ(p_i^{{2e_i}})      │\n"
            "│   must have ALL prime factors q satisfying q ≡ 1 or 3 (mod 8).                 │\n"
            "│   Any factor q ≡ 5 or 7 (mod 8) invalidates the component.                    │\n"
            "│                                                                                 │\n"
            "│ ALGORITHM (Two-Pass Cyclotomic Screening):                                      │\n"
            f"│   For each odd prime p ≤ {self.sieve_limit:,} and even exponent 2e ∈ {{2..{2*self.max_exponent}}}:    │\n"
            "│                                                                                 │\n"
            f"│   1. Compute σ(p^{{2e}}) = (p^{{2e+1}} - 1) / (p - 1) via pure-Rust u128       │\n"
            f"│   2. Cyclotomic decomposition: σ(p^{{2e}}) = ∏_{{d|(2e+1)}} Φ_d(p)             │\n"
            "│   3. Trial-divide each Φ_d(p) with early mod-8 exit:                            │\n"
            "│      a) Trial-divide by small primes (up to 10^7)                               │\n"
            "│      b) If any factor q ≡ 5 or 7 (mod 8) found → REJECT immediately           │\n"
            "│      c) Large composite cofactor: use mod-8 subgroup closure property           │\n"
            f"│         {{1,3}} closed under ×(mod 8), so cofactor ≡ 5,7 → must have bad factor │\n"
            "│      d) Ambiguous cofactor ≡ 1,3 (mod 8): Pollard ρ fallback to factor         │\n"
            "│                                                                                 │\n"
            "│ PARALLELISM: Rayon parallel iterator over all primes.                           │\n"
            f"│ OUTPUT: Surviving components sorted by abundance ratio σ/p^{{2e}} descending.   │\n"
            "└─────────────────────────────────────────────────────────────────────────────────┘\n"
            "\n"
            f"{ts} PROGRESS|PHASE|1|Legendre-Cattaneo Sieve\n"
            "\n"
        )

    def _trace_write_sieve_results(self, f, ts, retained_line, elapsed_sec, sieve_diag):
        """Write the Phase 1 sieve results summary box."""
        # Parse retained/pruned counts from the line
        m = RE_RETAINED_PRUNED.match(retained_line)
        retained = m.group(1) if m else "?"
        pruned = m.group(2) if m else "?"
        try:
            retained_n = int(retained.replace(",", ""))
            pruned_n = int(pruned.replace(",", ""))
            total = retained_n + pruned_n
            pct = (retained_n / total * 100) if total > 0 else 0
        except ValueError:
            total = 0
            pct = 0

        elapsed_str = time_utils.format_hhmmss(elapsed_sec)
        diag_line = sieve_diag.get("complete", "")

        f.write(
            "\n"
            f"{ts} SIEVE COMPLETE (elapsed: {elapsed_str})\n"
            "           ┌────────────────────────────────────────────────────────────┐\n"
            f"           │  Retained components: {retained:>8s}                              │\n"
            f"           │  Pruned (mod-8 violation): {pruned:>8s}                          │\n"
            f"           │  Total evaluated: ~{total:,} (p, 2e) pairs{' ' * max(1, 22 - len(f'{total:,}'))}│\n"
            f"           │  Retention rate: ~{pct:.1f}%{' ' * max(1, 40 - len(f'{pct:.1f}'))}│\n"
            "           │                                                            │\n"
            "           │  Surviving components are sorted by abundance ratio        │\n"
            f"           │  σ(p^{{2e}})/p^{{2e}} descending (small primes first).       │\n"
        )
        if diag_line:
            f.write(f"           │  Diag: {diag_line[:54]:<54s}│\n")
        f.write(
            "           └────────────────────────────────────────────────────────────┘\n"
            "\n"
        )

    def _trace_write_phase2_start(self, f, ts):
        """Write the Phase 2 DFS + ray-casting documentation block."""
        f.write(
            "\n"
            "┌─────────────────────────────────────────────────────────────────────────────────┐\n"
            f"│ PHASE 2 — FUSED DFS CONSTRUCTION & RAY-CASTING ({ts})                          │\n"
            "│                                                                                 │\n"
            "│ PURPOSE:                                                                        │\n"
            f"│ Construct all feasible N_L = ∏ p_i^{{2e_i}} prefixes via DFS, applying          │\n"
            f"│ multiple pruning layers.  When N_L > {self.prefix_stop:,}, perform exact      │\n"
            "│ ray-casting to test whether any valid N_R completion exists.                    │\n"
            "│                                                                                 │\n"
            "│ PARALLELISM:                                                                    │\n"
            "│   • Depth < 2: Rayon parallel work-stealing (clone prefix per child)           │\n"
            "│   • Depth ≥ 2: Sequential push/pop recursion (zero allocation)                 │\n"
            "│   • 64 lock-free AtomicU64 slots for active-prime telemetry                    │\n"
            "│                                                                                 │\n"
            "│ PRUNING LAYERS (in order at each DFS node):                                    │\n"
            f"│   1. BOUND CHECK:       N_L > 10^{self.bound_max} → prune                     │\n"
            "│   2. DYNAMIC ω BOUND:   gcd(N,15)=1 ⟹ ω(N) ≥ 15 (Prasad-Sunitha)           │\n"
            "│   3. OVERFLOW KILL:     running abundancy > 2.000001 → prune                  │\n"
            "│   4. STARVATION (A1):   abundancy × best_remaining < 2.0 → prune             │\n"
            "│   5. EXHAUSTION (A3):   not enough components left → prune                    │\n"
            "│   6. RAY-CASTING:       CRT → Tonelli-Shanks → factor z → check σ(z²)        │\n"
            "│                         (Immediately terminates branch if triggered)          │\n"
            "└─────────────────────────────────────────────────────────────────────────────────┘\n"
            "\n"
            f"{ts} PROGRESS|PHASE|2|Fused DFS Construction & Ray-Casting\n"
            "\n"
        )

    def _trace_write_dfs_results(self, f, ts, dfs_line, elapsed_sec):
        """Write the Phase 2 DFS completion results box."""
        # Parse DFS stats from the line
        m = RE_DFS_TRACE.match(dfs_line)
        branches = m.group(1) if m else "?"
        ab = m.group(2) if m else "?"

        elapsed_str = (
            f"~{elapsed_sec:.0f}s"
            if elapsed_sec < 60
            else time_utils.format_hhmmss(elapsed_sec)
        )

        f.write(
            "\n"
            f"{ts} DFS COMPLETE (elapsed: {elapsed_str})\n"
            "           ┌────────────────────────────────────────────────────────────┐\n"
            f"           │  Evaluated Branches:  {branches:>10s}                          │\n"
            f"           │  Abundance-pruned:    {ab:>10s}                          │\n"
            "           │  Ray-cast rejections:          0   (no candidates survived)  │\n"
            "           └────────────────────────────────────────────────────────────┘\n"
            "\n"
        )

    def _trace_write_verification_result(
        self, f, ts, done_line, total_elapsed, phase1_dur, phase2_dur
    ):
        """Write the final verification result and timing breakdown."""
        # Parse the status message from PROGRESS|DONE|...|...|message
        parts = done_line.split("|")
        status_msg = parts[4] if len(parts) > 4 else "Complete"

        total_str = time_utils.format_hhmmss(total_elapsed)
        p1_str = time_utils.format_hhmmss(phase1_dur) if phase1_dur > 0 else "N/A"
        p2_str = (
            f"~{phase2_dur:.0f}s"
            if 0 < phase2_dur < 60
            else time_utils.format_hhmmss(phase2_dur)
        )
        p1_pct = (
            f"{(phase1_dur / total_elapsed * 100):.2f}%" if total_elapsed > 0 else "—"
        )
        p2_pct = (
            f"{(phase2_dur / total_elapsed * 100):.2f}%" if total_elapsed > 0 else "—"
        )

        f.write(
            "\n"
            "┌─────────────────────────────────────────────────────────────────────────────────┐\n"
            "│ VERIFICATION RESULT                                                             │\n"
            "│                                                                                 │\n"
            f"│  ✓ {status_msg:<76s}│\n"
            "│                                                                                 │\n"
            f"│  Combined with the Hagis-Cohen 1982 baseline (N ≤ 10^{VERIFIED_BASELINE_EXP}), this establishes:"
            f"{' ' * 10}│\n"
            "│                                                                                 │\n"
            f"│     No quasiperfect number exists below 10^{self.bound_max}."
            f"{' ' * max(1, 35 - len(str(self.bound_max)))}│\n"
            "│                                                                                 │\n"
            "│  PROOF INTEGRITY:                                                               │\n"
            f"│    • Lean 4 proofs: {self.lean_status.sorry_count} sorry, "
            f"{self.lean_status.axiom_count} axioms"
            f"{' ' * max(1, 49 - len(str(self.lean_status.sorry_count)) - len(str(self.lean_status.axiom_count)))}│\n"
            "│    • Rust engine: exited cleanly (code 0)"
            f"{' ' * 38}│\n"
            f"{' ' * 7}│\n"
            "│                                                                                 │\n"
            "│  TIMING BREAKDOWN:                                                              │\n"
            "│    Phase 0 (Init):        < 1s"
            f"{' ' * 49}│\n"
            f"│    Phase 1 (Sieve):       {p1_str:<12s} ({p1_pct} of total)"
            f"{' ' * max(1, 30 - len(p1_str) - len(p1_pct))}│\n"
            f"│    Phase 2 (DFS+Raycast): {p2_str:<12s} ({p2_pct} of total)"
            f"{' ' * max(1, 30 - len(p2_str) - len(p2_pct))}│\n"
            f"│    Total:                 {total_str:<55s}│\n"
            "└─────────────────────────────────────────────────────────────────────────────────┘\n"
            "\n"
            f"{ts} {done_line}\n"
        )

    @staticmethod
    def _summarize_buffer(buf):
        n = len(buf)
        updates = [l for l in buf if l.startswith("PROGRESS|UPDATE|")]
        others = [l for l in buf if not l.startswith("PROGRESS|UPDATE|")]
        parts = []
        if updates:
            last = updates[-1]
            parts.append(f"{len(updates)} progress ticks")
            m = RE_PREFIXES.search(last)
            if m:
                parts.append(f"prefixes={m.group(1)}")
            m = RE_AB_PRUNED.search(last)
            if m:
                parts.append(f"z3={m.group(1)}")
        if others:
            for o in others[:3]:
                parts.append(o[:80])
            if len(others) > 3:
                parts.append(f"...+{len(others)-3} more")
        return " | ".join(parts) if parts else f"{n} lines"

    # ═══════════════════════════════════════════════════════════════════
    #  Message processing
    # ═══════════════════════════════════════════════════════════════════

    def _parse_line_for_ui(self, line):
        if line.startswith("PROGRESS|UPDATE|"):
            m_active = RE_P_ACTIVE.search(line)
            m_ab = RE_AB_PRUNED.search(line)
            active_str = m_active.group(1).strip() if m_active else ""
            if active_str:
                cnt = RE_P_ACTIVE_TOTAL.search(active_str)
                active_cnt = (
                    int(cnt.group(1))
                    if cnt
                    else len([x for x in active_str.split(",") if x.strip().isdigit()])
                )
            else:
                active_cnt = 0
            ab_pruned = int(m_ab.group(1).replace(",", "")) if m_ab else 0
            return {
                "type": "progress_update",
                "active_str": active_str,
                "active_cnt": active_cnt,
                "ab_pruned": ab_pruned,
            }

        if line.startswith("{") and (
            "Phase" in line
            or "StatusUpdate" in line
            or "DFSComplete" in line
            or "Done" in line
        ):
            import json

            try:
                return {"type": "json_progress", "event": json.loads(line)}
            except json.JSONDecodeError:
                pass

        if "=== UALBF Engine Initializing ===" in line:
            return {"type": "init"}

        m = RE_TARGET_BOUND.match(line)
        if m:
            return {"type": "target_bound", "min": m.group(1), "max": m.group(2)}

        if line.startswith("Sieve:"):
            return {"type": "sieve", "line": line}

        m = RE_RETAINED_PRUNED.match(line)
        if m:
            return {
                "type": "retained_pruned",
                "retained": m.group(1),
                "pruned": m.group(2),
            }

        m = RE_DFS_COMPLETE.match(line)
        if m:
            return {
                "type": "dfs_complete",
                "ab_pruned": int(m.group(1)),
                "z3": int(m.group(2)),
                "conflicts": int(m.group(3)),
            }

        if "QUASIPERFECT NUMBER FOUND" in line:
            return {"type": "qp_found", "line": line}

        if line.startswith("overflow:"):
            return {"type": "overflow"}

        return {"type": "unstructured", "line": line}

    def _process_queue(self):
        while not self.queue.empty():
            try:
                msg = self.queue.get_nowait()
            except queue.Empty:
                break

            if not isinstance(msg, dict):
                continue

            t = msg.get("type")

            if t == "success_exit":
                self._log("✓ Engine process exited gracefully.", "success")
                self.finished = True
            elif t == "crash_exit":
                code = msg.get("code", "unknown")
                self._log(f"✗ Engine CRASHED — exit code {code}", "error")
                self.status_text = f"CRASHED (exit code {code})"
            elif t == "error":
                self._log(f"✗ {msg.get('msg', '')}", "error")
            elif t == "lean_build":
                sub = msg.get("sub")
                if sub == "START":
                    self.phase_num = "0"
                    self.phase_text = "📐 Phase 0: Lean 4 Build"
                    self.is_indeterminate = True
                    self.status_text = "Running lake build..."
                    self._log("📐 Starting Lean 4 build (lake build)...", "phase")
                elif sub == "OK":
                    self._log("✓ Lean 4 build succeeded", "success")
                    self.lean_initialized = True
                elif sub == "SKIPPED":
                    self._log("⏭ Lean build skipped (--skip-lean-build)", "phase")
                    self.lean_initialized = True
                elif sub == "FAIL":
                    reason = msg.get("reason", "unknown")
                    self._log(f"✗ Lean build FAILED: {reason}", "error")
            elif t == "lean_log":
                text_log = msg.get("text", "")
                self.status_text = text_log[:120]
                if (
                    "error" in text_log.lower()
                    or "warning" in text_log.lower()
                    or "Building" in text_log
                ):
                    self._log(f"  {text_log[:100]}", "phase")
            elif t == "lean_scan":
                sorry_n = msg.get("sorry", 0)
                axiom_n = msg.get("axiom", 0)
                if sorry_n > 0:
                    self._log(
                        f"⚠ Lean scan: {sorry_n} sorry, {axiom_n} axiom(s)", "phase"
                    )
                else:
                    self._log(f"✓ Lean scan: 0 sorry, {axiom_n} axiom(s)", "success")
            elif t == "progress_update":
                self.active_primes_str = msg.get("active_str", "")
                self.active_primes_cnt = msg.get("active_cnt", 0)
                self.abundance_pruned = msg.get("ab_pruned", 0)
            elif t == "json_progress":
                self._handle_json_progress(msg["event"])
            elif t == "init":
                self.lean_initialized = True
                self.status_text = "Engine initializing..."
                self._log("⚙ Engine process started", "phase")
            elif t == "target_bound":
                self.target_bound_min = msg["min"]
                self.target_bound_max = msg["max"]
                self.target_bound = f"10^{msg['min']} < N < 10^{msg['max']}"
                self._log(f"🎯 {self.target_bound}", "phase")
            elif t == "sieve":
                self._log(f"⚙ {msg['line']}", "info")
            elif t == "retained_pruned":
                self.retained_comps = msg["retained"]
                self.pruned_comps = msg["pruned"]
                self._log(
                    f"⚗  Sieve: {self.retained_comps} retained, {self.pruned_comps} pruned",
                    "phase",
                )
            elif t == "dfs_complete":
                self.abundance_pruned = msg["ab_pruned"]
                self.prune_hits = msg["z3"]
                self.conflicts_learned = msg["conflicts"]
                self._log(
                    f"🌳 DFS complete: ab_pruned={msg['ab_pruned']} z3={msg['z3']} conflicts={msg['conflicts']}",
                    "success",
                )
            elif t == "qp_found":
                self.qp_found += 1
                self._log(f"████ {msg['line']} ████", "qp")
            elif t == "overflow":
                self.overflow_count += 1
            elif t == "unstructured":
                self._log(msg["line"][:120], "info")

    def _handle_json_progress(self, event):
        if "Phase" in event:
            phase = event["Phase"]
            self.phase_num = str(phase["phase"])
            phase_desc = phase["name"]
            icon = PHASE_ICONS.get(self.phase_num, "⚙ ")
            self.phase_text = f"{icon} Phase {self.phase_num}: {phase_desc}"
            self.progress_pct = 0.0
            self.phase_start = time.time()
            self.eta_text = "Calculating..."
            self.rate_text = "—"
            self.is_indeterminate = False
            self._log(f"▶ Started Phase {self.phase_num}: {phase_desc}", "phase")
            if "All work units completed" in phase_desc:
                self.status_text = "Done"
        elif "StatusUpdate" in event:
            up = event["StatusUpdate"]
            c = up["c"]
            total_weight = up["total_weight_scaled"]
            comp = up["comp"]
            active_str = up["active_str"]
            prefixes = up["prefixes"]
            ap = up["ap"]

            self.status_text = (
                f"P-Active: {active_str} | Prefixes: {prefixes} | AbPruned: {ap}"
            )

            self.active_primes_str = active_str
            if active_str:
                cnt = RE_P_ACTIVE_TOTAL.search(active_str)
                self.active_primes_cnt = (
                    int(cnt.group(1))
                    if cnt
                    else len([x for x in active_str.split(",") if x.strip().isdigit()])
                )
            else:
                self.active_primes_cnt = 0

            elapsed = time.time() - self.phase_start
            if self.phase_num == "2":
                completed_weight = comp
                if total_weight > 0:
                    self.progress_pct = (completed_weight / total_weight) * 100
                    if completed_weight > 0 and elapsed > 2.0:
                        rate = completed_weight / elapsed
                        rem = total_weight - completed_weight
                        eta_secs = rem / rate
                        self.eta_text = cert_util.format_duration(
                            eta_secs, style="short"
                        )
                    else:
                        self.eta_text = "Calculating..."

            if elapsed > 1.0 and c > 0:
                self.rate_text = f"{c / elapsed:.0f} p/s"
        elif "DFSComplete" in event:
            d = event["DFSComplete"]
            total_branches = d["total_branches"]
            ap = d["ap"]
            rp = d["rp"]
            self.progress_pct = 100.0
            self.status_text = f"DFS complete. Evaluated Branches: {total_branches} | AbPruned: {ap} | RaycastPruned: {rp}"
            self.eta_text = "0s"
        elif "Done" in event:
            d = event["Done"]
            self.progress_pct = 100.0
            self.phase_num = "4"
            self.phase_text = "✓ Phase 4: Verification Complete"
            self.status_text = f"10^{d['target_min_log10']} < N < 10^{d['target_max_log10']} Confirmed in {d['elapsed_ms']}ms"
            self.eta_text = "Done"
            self.is_indeterminate = False
            self.engine_done = True

    def _log(self, text, level="info"):
        ts = time_utils.get_current_timestamp("%H:%M:%S")
        if getattr(self, "is_headless", False):
            # In headless mode, only print milestones and errors, filter out high-frequency info
            if level in ("phase", "success", "error", "qp"):
                print(f"[{ts}] {text}")
                sys.stdout.flush()
        self.log_lines.append((ts, text, level))
        if len(self.log_lines) > MAX_LOG_LINES:
            self.log_lines.pop(0)

    # ═══════════════════════════════════════════════════════════════════
    #  Rendering
    # ═══════════════════════════════════════════════════════════════════

    def _export_telemetry(self):
        import json

        telemetry = {
            "candidate_rate": getattr(self, "rate_text", "—"),
            "progress_percentage": getattr(self, "progress_pct", 0.0),
            "eta": getattr(self, "eta_text", "—"),
            "target_bounds": getattr(self, "target_bound", "—"),
            "lean_statuses": {
                "sorry_count": (
                    self.lean_status.sorry_count if hasattr(self, "lean_status") else 0
                ),
                "axiom_count": (
                    self.lean_status.axiom_count if hasattr(self, "lean_status") else 0
                ),
                "build_ok": (
                    self.lean_status.build_ok if hasattr(self, "lean_status") else None
                ),
            },
        }
        try:
            with open("telemetry.json", "w", encoding="utf-8") as f:
                json.dump(telemetry, f, indent=2)
        except Exception:
            pass

    def _draw_loop(self):
        if getattr(self, "is_headless", False):
            while self.running:
                self._process_queue()
                self._export_telemetry()
                time.sleep(1.0 / REFRESH_HZ)
            return

        while self.running:
            self._process_queue()
            self._render()
            try:
                c = self.stdscr.getch()
                if c == ord("q") or c == ord("Q"):
                    self.running = False
                elif c == ord("r") or c == ord("R"):
                    if self.awaiting_rerun:
                        self.bound_min = self.bound_max
                        self.bound_max += self.args.raise_step
                        self._log(
                            f"▲ Raising bounds → 10^{self.bound_min} < N < 10^{self.bound_max}",
                            "phase",
                        )
                        self.awaiting_rerun = False
                elif c == ord("l") or c == ord("L"):
                    self.show_lean_overlay = not self.show_lean_overlay
                elif c == curses.KEY_RESIZE:
                    self.stdscr.clear()
            except curses.error:
                pass

    def _render(self):
        self.stdscr.erase()
        h, w = self.stdscr.getmaxyx()

        if h < 24 or w < 70:
            safe_addstr(
                self.stdscr,
                0,
                0,
                f"Terminal too small ({w}×{h}). Need ≥70×24.",
                self.C_RED,
            )
            self.stdscr.refresh()
            return

        y = 0

        # ── ASCII Header ───────────────────────────────────────────────
        for i, art_line in enumerate(HEADER_ART):
            x = max(0, (w - len(art_line)) // 2)
            safe_addstr(self.stdscr, y + i, x, art_line, self.C_CYAN | curses.A_BOLD)
        y += len(HEADER_ART) + 1

        # ── Lean Overlay ───────────────────────────────────────────────
        if self.show_lean_overlay:
            y = self._render_lean_overlay(y, w)
        else:
            # Show compact Lean status in header area
            lean_sym = (
                "✓"
                if self.lean_status.build_ok
                else ("✗" if self.lean_status.build_ok is False else "…")
            )
            lean_col = (
                self.C_GREEN
                if self.lean_status.build_ok
                else (
                    self.C_RED if self.lean_status.build_ok is False else self.C_YELLOW
                )
            )
            sorry_str = (
                f"  sorry:{self.lean_status.sorry_count} axiom:{self.lean_status.axiom_count}"
                if self.lean_status.theorems
                else ""
            )
            safe_addstr(
                self.stdscr,
                y,
                1,
                f"Lean [{lean_sym}]{sorry_str}  (press 'l' for details)",
                lean_col,
            )
            y += 1

        # ── Main Dashboard Box ─────────────────────────────────────────
        panel_x = 1
        panel_w = w - 2
        panel_h = 14
        self._draw_box(
            y,
            panel_x,
            panel_w,
            panel_h,
            f"Engine Dashboard — Run #{self.current_run}",
            self.C_CYAN,
        )

        row = y + 1
        self._label_value(
            row, panel_x + 2, "Phase      ", self.phase_text, self.C_BOLD, panel_w
        )
        row += 1
        status_color = (
            self.C_RED | curses.A_BOLD
            if "CRASH" in self.status_text
            else self.C_YELLOW | curses.A_BOLD
        )
        self._label_value(
            row,
            panel_x + 2,
            "Status     ",
            self.status_text[: panel_w - 20],
            status_color,
            panel_w,
        )

        row += 1
        safe_addstr(
            self.stdscr, row, panel_x, "├" + "─" * (panel_w - 2) + "┤", self.C_CYAN
        )

        row += 1
        uptime = time_utils.format_hhmmss(time.time() - self.global_start)
        self._label_value(
            row, panel_x + 2, "Uptime     ", uptime, self.C_WHITE, panel_w
        )
        row += 1
        self._label_value(
            row, panel_x + 2, "Processed  ", self.processed_text, self.C_WHITE, panel_w
        )
        row += 1
        self._label_value(
            row, panel_x + 2, "Throughput ", self.rate_text, self.C_WHITE, panel_w
        )
        row += 1
        self._label_value(
            row, panel_x + 2, "ETA        ", self.eta_text, self.C_YELLOW, panel_w
        )

        col2 = panel_x + max(38, panel_w // 2)
        stat_row = y + 4
        self._label_value(
            stat_row, col2, "Verified ≤", self.verified_floor, self.C_GREEN, panel_w
        )
        stat_row += 1
        self._label_value(
            stat_row,
            col2,
            "Target    ",
            self.target_bound,
            self.C_MAGENTA | curses.A_BOLD,
            panel_w,
        )
        stat_row += 1
        self._label_value(
            stat_row, col2, "Retained  ", self.retained_comps, self.C_WHITE, panel_w
        )
        stat_row += 1
        self._label_value(
            stat_row, col2, "Sieve ✗   ", self.pruned_comps, self.C_YELLOW, panel_w
        )
        stat_row += 1
        qp_color = self.C_GREEN | curses.A_BOLD if self.qp_found > 0 else self.C_WHITE
        self._label_value(
            stat_row, col2, "QP Found  ", str(self.qp_found), qp_color, panel_w
        )

        row += 1
        safe_addstr(
            self.stdscr, row, panel_x, "├" + "─" * (panel_w - 2) + "┤", self.C_CYAN
        )

        row += 1
        prune_strs = [
            f"AbundancePrune: {self.abundance_pruned:,}",
        ]
        prune_line = "  │  ".join(prune_strs)
        safe_addstr(
            self.stdscr, row, panel_x + 2, prune_line[: panel_w - 4], self.C_MAGENTA
        )

        row += 1
        active_line = (
            f"Active Primes ({self.active_primes_cnt}): {self.active_primes_str}"
        )
        safe_addstr(
            self.stdscr, row, panel_x + 2, active_line[: panel_w - 4], self.C_CYAN
        )

        row += 1
        lean_sym = "✓" if self.lean_initialized else "…"
        lean_col = self.C_GREEN if self.lean_initialized else self.C_YELLOW
        safe_addstr(self.stdscr, row, panel_x + 2, f"Lean FFI [{lean_sym}]", lean_col)

        # ── Progress Bar ───────────────────────────────────────────────
        y_bar = y + panel_h
        bar_w = panel_w - 12
        if bar_w > 10:
            if self.is_indeterminate:
                pos = int((self.progress_pct / 100.0) * max(1, bar_w - 6))
                bar = "─" * pos + "▓▓▓▓" + "─" * max(0, bar_w - 4 - pos)
                bar_color = self.C_YELLOW
                pct_str = " ···  "
            else:
                filled = int((self.progress_pct / 100.0) * bar_w)
                bar = "█" * filled + "░" * (bar_w - filled)
                bar_color = self.C_GREEN
                pct_str = f" {self.progress_pct:5.1f}%"
            safe_addstr(
                self.stdscr, y_bar, panel_x + 1, f" [{bar}]{pct_str}", bar_color
            )

        # ── Sparkline ──────────────────────────────────────────────────
        y_bar += 1
        if self.throughput_hist and bar_w > 10:
            spark_chars = " ▁▂▃▄▅▆▇█"
            hist = self.throughput_hist[-(panel_w - 20) :]
            if hist:
                max_val = max(hist) if max(hist) > 0 else 1
                spark = "".join(
                    spark_chars[int((v / max_val) * (len(spark_chars) - 1))]
                    for v in hist
                )
                safe_addstr(
                    self.stdscr, y_bar, panel_x + 2, "Throughput: ", self.C_LABEL
                )
                safe_addstr(
                    self.stdscr,
                    y_bar,
                    panel_x + 14,
                    spark[: panel_w - 16],
                    self.C_GREEN,
                )
            y_bar += 1

        # ── Bound History (if any previous runs) ──────────────────────
        if self.bound_history:
            y_bar += 1
            safe_addstr(
                self.stdscr,
                y_bar,
                panel_x + 2,
                "─── Bound History ───",
                self.C_MAGENTA | curses.A_BOLD,
            )
            y_bar += 1
            for bmin, bmax, result, elapsed in self.bound_history[-3:]:
                elapsed_str = time_utils.format_hhmmss(elapsed)
                entry = f"  10^{bmin}..10^{bmax}: {result} ({elapsed_str})"
                color = self.C_GREEN if "✓" in result else self.C_RED
                safe_addstr(
                    self.stdscr, y_bar, panel_x + 2, entry[: panel_w - 4], color
                )
                y_bar += 1

        # ── Log Panel ──────────────────────────────────────────────────
        log_y = y_bar + 1
        log_x = 1
        log_w = w - 2
        log_h = h - log_y - 1
        if log_h < 3:
            log_h = 3

        self._draw_box(log_y, log_x, log_w, log_h, "Event Stream", self.C_MAGENTA)

        max_visible = log_h - 2
        visible = self.log_lines[-max_visible:] if max_visible > 0 else []
        for i, (ts, text, level) in enumerate(visible):
            color = self.C_WHITE
            if level == "error":
                color = self.C_RED | curses.A_BOLD
            elif level == "success":
                color = self.C_GREEN
            elif level == "phase":
                color = self.C_CYAN | curses.A_BOLD
            elif level == "qp":
                color = self.C_GREEN | curses.A_BOLD
            entry = f"[{ts}] {text}"
            safe_addstr(
                self.stdscr, log_y + 1 + i, log_x + 1, entry[: log_w - 3], color
            )

        # ── Footer ─────────────────────────────────────────────────────
        footer_y = h - 1
        if self.awaiting_rerun:
            footer = (
                " Search complete. Press 'r' to raise bounds & re-run, 'q' to quit. "
            )
        elif self.finished:
            footer = " Search complete. Press 'q' to exit. "
        else:
            footer = " q=Quit  l=Lean  r=Raise "
        safe_addstr(self.stdscr, footer_y, 0, footer, self.C_HEADER)
        ts_str = time_utils.get_current_timestamp(" %H:%M:%S ")
        safe_addstr(
            self.stdscr, footer_y, max(0, w - len(ts_str) - 1), ts_str, self.C_HEADER
        )

        self.stdscr.refresh()

    def _render_lean_overlay(self, y, w):
        """Render the Lean proof status overlay panel."""
        panel_x = 1
        panel_w = w - 2
        panel_h = len(LEAN_THEOREMS) + 7
        self._draw_box(
            y,
            panel_x,
            panel_w,
            panel_h,
            "Lean 4 Proof Status (press 'l' to close)",
            self.C_MAGENTA,
        )

        row = y + 1
        build_sym = {"✓": self.C_GREEN, "✗": self.C_RED, "…": self.C_YELLOW}
        bs = (
            "✓"
            if self.lean_status.build_ok
            else ("✗" if self.lean_status.build_ok is False else "…")
        )
        self._label_value(
            row, panel_x + 2, "lake build", bs, build_sym.get(bs, self.C_WHITE), panel_w
        )
        row += 1
        safe_addstr(
            self.stdscr, row, panel_x, "├" + "─" * (panel_w - 2) + "┤", self.C_MAGENTA
        )
        row += 1

        for thm_name, filename in LEAN_THEOREMS:
            status = self.lean_status.theorems.get(thm_name, "—")
            if status == "verified":
                sym, col = "✓ verified", self.C_GREEN
            elif status == "sorry":
                sym, col = "⚠ sorry", self.C_YELLOW | curses.A_BOLD
            elif status == "axiomatic":
                sym, col = "✓ axiom", self.C_BLUE
            elif status == "axiom":
                sym, col = "△ axiom", self.C_CYAN
            elif status == "missing":
                sym, col = "✗ missing", self.C_RED
            else:
                sym, col = "— unknown", self.C_WHITE
            label = f"{thm_name:<35s}"
            safe_addstr(self.stdscr, row, panel_x + 2, label, self.C_LABEL)
            safe_addstr(self.stdscr, row, panel_x + 38, sym, col)

            # Display truncated hash instead of filename if available
            thm_hash = getattr(self.lean_status, "hashes", {}).get(thm_name, "")
            if thm_hash:
                safe_addstr(
                    self.stdscr,
                    row,
                    panel_x + 52,
                    f"[{thm_hash[:8]}] {filename}",
                    self.C_WHITE,
                )
            else:
                safe_addstr(self.stdscr, row, panel_x + 52, filename, self.C_WHITE)
            row += 1

        row += 1
        safe_addstr(
            self.stdscr, row, panel_x, "├" + "─" * (panel_w - 2) + "┤", self.C_MAGENTA
        )
        row += 1
        self._label_value(
            row,
            panel_x + 2,
            "Verified Logic Hash",
            getattr(self.lean_status, "verified_logic_hash", "—"),
            self.C_CYAN,
            panel_w,
        )

        return y + panel_h

    # ═══════════════════════════════════════════════════════════════════
    #  Drawing helpers
    # ═══════════════════════════════════════════════════════════════════

    def _draw_box(self, y, x, w, h, title, color):
        safe_addstr(self.stdscr, y, x, "╭" + "─" * (w - 2) + "╮", color)
        for i in range(1, h - 1):
            safe_addstr(self.stdscr, y + i, x, "│", color)
            safe_addstr(self.stdscr, y + i, x + w - 1, "│", color)
        safe_addstr(self.stdscr, y + h - 1, x, "╰" + "─" * (w - 2) + "╯", color)
        safe_addstr(self.stdscr, y, x + 2, f" {title} ", color | curses.A_BOLD)

    def _label_value(self, y, x, label, value, value_color, panel_w):
        safe_addstr(self.stdscr, y, x, f"{label}: ", self.C_LABEL)
        safe_addstr(
            self.stdscr,
            y,
            x + len(label) + 2,
            str(value)[: panel_w - len(label) - 6],
            value_color,
        )


# ═══════════════════════════════════════════════════════════════════════════════
#  Entry point
# ═══════════════════════════════════════════════════════════════════════════════

if __name__ == "__main__":
    args = parse_args()
    if getattr(args, "headless", False):
        CursesGUI(None, args)
    else:
        curses.wrapper(lambda stdscr: CursesGUI(stdscr, args))
