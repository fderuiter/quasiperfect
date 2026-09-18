"""
Modular unit and headless pipeline integration tests for terminal dashboard (run_gui.py).
"""

import json
import sys
from pathlib import Path

# Ensure rust-engine directory is in sys.path
rust_engine_dir = Path(__file__).parent.parent / "rust-engine"
if str(rust_engine_dir) not in sys.path:
    sys.path.insert(0, str(rust_engine_dir))

import run_gui  # noqa: E402

# ==============================================================================
# 1. CLI Argument Parsing Unit Tests
# ==============================================================================


def test_parse_args_defaults(monkeypatch):
    """Test parse_args default parameter values."""
    monkeypatch.setattr(sys, "argv", ["run_gui.py"])
    args = run_gui.parse_args()

    assert args.min == run_gui.VERIFIED_BASELINE_EXP
    assert args.max == run_gui._MAX_EXP
    assert args.sieve_limit == run_gui._SIEVE_LIMIT
    assert args.max_exponent == run_gui._MAX_EXPONENT
    assert args.prefix_stop == run_gui._PREFIX_STOP
    assert args.auto_raise is False
    assert args.raise_step == 2
    assert args.skip_lean_build is False
    assert args.debug is False
    assert args.headless is False


def test_parse_args_overrides(monkeypatch):
    """Test parse_args with custom CLI argument overrides."""
    custom_args = [
        "run_gui.py",
        "--min",
        "40",
        "--max",
        "45",
        "--sieve-limit",
        "500000",
        "--max-exponent",
        "6",
        "--prefix-stop",
        "200000000000",
        "--auto-raise",
        "--raise-step",
        "5",
        "--skip-lean-build",
        "--debug",
        "--headless",
    ]
    monkeypatch.setattr(sys, "argv", custom_args)
    args = run_gui.parse_args()

    assert args.min == 40
    assert args.max == 45
    assert args.sieve_limit == 500000
    assert args.max_exponent == 6
    assert args.prefix_stop == 200000000000
    assert args.auto_raise is True
    assert args.raise_step == 5
    assert args.skip_lean_build is True
    assert args.debug is True
    assert args.headless is True


# ==============================================================================
# 2. LeanProofStatus Unit Tests
# ==============================================================================


def test_lean_proof_status_scan(tmp_path, monkeypatch):
    """Test LeanProofStatus theorem scanning and checksum extraction."""
    manifest_data = {
        "verified_logic_hash": "a1b2c3d4e5f67890" * 4,
        "theorems": [
            {
                "name": "UALBF.QPN.qpn_is_odd_square",
                "status": "verified",
                "checksum": "chk_verified_123",
            },
            {
                "name": "UALBF.QPN.sorry_thm",
                "status": "sorry",
                "checksum": "chk_sorry_456",
            },
            {
                "name": "UALBF.QPN.unverified_thm",
                "status": "unverified",
                "checksum": "chk_unverified_789",
            },
            {
                "name": "UALBF.QPN.axiom_thm",
                "status": "axiom",
                "checksum": "chk_axiom_012",
            },
            {
                "name": "UALBF.QPN.axiomatic_thm",
                "status": "axiomatic",
                "checksum": "chk_axiomatic_345",
            },
        ],
    }
    manifest_file = tmp_path / "proof_manifest.json"
    manifest_file.write_text(json.dumps(manifest_data), encoding="utf-8")

    monkeypatch.setenv("UALBF_PROOF_MANIFEST", str(manifest_file))

    lps = run_gui.LeanProofStatus(str(tmp_path))
    lps.scan()

    assert lps.verified_logic_hash == "a1b2c3d4e5f67890" * 4
    assert lps.theorems["qpn_is_odd_square"] == "verified"
    assert lps.theorems["sorry_thm"] == "sorry"
    assert lps.theorems["unverified_thm"] == "sorry"  # unverified -> sorry
    assert lps.theorems["axiom_thm"] == "axiom"
    assert lps.theorems["axiomatic_thm"] == "axiomatic"

    assert lps.sorry_count == 2
    assert lps.axiom_count == 2
    assert lps.hashes["qpn_is_odd_square"] == "chk_verified_123"


def test_lean_proof_status_missing_manifest(tmp_path, monkeypatch):
    """Test LeanProofStatus handling when proof_manifest.json is missing or corrupted."""
    missing_file = tmp_path / "non_existent_manifest.json"
    monkeypatch.setenv("UALBF_PROOF_MANIFEST", str(missing_file))

    lps = run_gui.LeanProofStatus(str(tmp_path))
    lps.scan()

    assert lps.sorry_count == 0
    assert lps.axiom_count == 0
    assert lps.theorems == {}

    # Corrupted manifest
    corrupt_file = tmp_path / "corrupt_manifest.json"
    corrupt_file.write_text("invalid json {", encoding="utf-8")
    monkeypatch.setenv("UALBF_PROOF_MANIFEST", str(corrupt_file))

    lps_corrupt = run_gui.LeanProofStatus(str(tmp_path))
    lps_corrupt.scan()

    assert lps_corrupt.sorry_count == 0
    assert lps_corrupt.axiom_count == 0


# ==============================================================================
# 3. Regex Log Extraction Unit Tests (_parse_line_for_ui)
# ==============================================================================


def test_parse_line_for_ui_patterns():
    """Test regex log extraction in _parse_line_for_ui against engine output lines."""
    # Progress update tick with (N total) format
    line_update = (
        "PROGRESS|UPDATE| P-Active: 3, 5, 7 (3 total) | Prefixes: 123 | AbPruned: 456"
    )
    res_update = run_gui.CursesGUI._parse_line_for_ui(None, line_update)
    assert res_update["type"] == "progress_update"
    assert res_update["active_str"] == "3, 5, 7 (3 total)"
    assert res_update["active_cnt"] == 3
    assert res_update["ab_pruned"] == 456

    # Progress update tick without (N total) format
    line_update_no_total = (
        "PROGRESS|UPDATE| P-Active: 3, 5, 7, 11 | Prefixes: 100 | AbPruned: 200"
    )
    res_update_no_total = run_gui.CursesGUI._parse_line_for_ui(
        None, line_update_no_total
    )
    assert res_update_no_total["type"] == "progress_update"
    assert res_update_no_total["active_str"] == "3, 5, 7, 11"
    assert res_update_no_total["active_cnt"] == 4
    assert res_update_no_total["ab_pruned"] == 200

    # JSON progress events
    line_json_phase = '{"Phase": {"phase": 1, "name": "Legendre-Cattaneo Sieve"}}'
    res_json_phase = run_gui.CursesGUI._parse_line_for_ui(None, line_json_phase)
    assert res_json_phase["type"] == "json_progress"
    assert res_json_phase["event"]["Phase"]["phase"] == 1

    line_json_status = '{"StatusUpdate": {"c": 100, "total_weight_scaled": 1000, "comp": 50, "active_str": "3, 5", "prefixes": 10, "ap": 5}}'
    res_json_status = run_gui.CursesGUI._parse_line_for_ui(None, line_json_status)
    assert res_json_status["type"] == "json_progress"
    assert res_json_status["event"]["StatusUpdate"]["c"] == 100

    line_json_dfs = '{"DFSComplete": {"total_branches": 200, "ap": 150, "rp": 20}}'
    res_json_dfs = run_gui.CursesGUI._parse_line_for_ui(None, line_json_dfs)
    assert res_json_dfs["type"] == "json_progress"
    assert res_json_dfs["event"]["DFSComplete"]["total_branches"] == 200

    line_json_done = (
        '{"Done": {"target_min_log10": 35, "target_max_log10": 37, "elapsed_ms": 1234}}'
    )
    res_json_done = run_gui.CursesGUI._parse_line_for_ui(None, line_json_done)
    assert res_json_done["type"] == "json_progress"
    assert res_json_done["event"]["Done"]["target_min_log10"] == 35

    # Corrupted JSON starting with {
    line_json_corrupt = '{"Phase": invalid_json_syntax'
    res_json_corrupt = run_gui.CursesGUI._parse_line_for_ui(None, line_json_corrupt)
    assert res_json_corrupt["type"] == "unstructured"

    # Init marker
    res_init = run_gui.CursesGUI._parse_line_for_ui(
        None, "=== UALBF Engine Initializing ==="
    )
    assert res_init["type"] == "init"

    # Target bound
    line_target = "Target Bound: 10^35 < N < 10^37"
    res_target = run_gui.CursesGUI._parse_line_for_ui(None, line_target)
    assert res_target["type"] == "target_bound"
    assert res_target["min"] == "35"
    assert res_target["max"] == "37"

    # Sieve configuration
    line_sieve = "Sieve: Limit=250000"
    res_sieve = run_gui.CursesGUI._parse_line_for_ui(None, line_sieve)
    assert res_sieve["type"] == "sieve"
    assert res_sieve["line"] == "Sieve: Limit=250000"

    # Retained/Pruned
    line_rp = "Retained: 1,234, Pruned: 5,678"
    res_rp = run_gui.CursesGUI._parse_line_for_ui(None, line_rp)
    assert res_rp["type"] == "retained_pruned"
    assert res_rp["retained"] == "1,234"
    assert res_rp["pruned"] == "5,678"

    # DFS complete text
    line_dfs = "DFS complete. Abundance-pruned: 100 | Topological-pruned: 200 | Conflicts learned: 50"
    res_dfs = run_gui.CursesGUI._parse_line_for_ui(None, line_dfs)
    assert res_dfs["type"] == "dfs_complete"
    assert res_dfs["ab_pruned"] == 100
    assert res_dfs["z3"] == 200
    assert res_dfs["conflicts"] == 50

    # Quasiperfect discovery
    line_qp = "QUASIPERFECT NUMBER FOUND N=12345"
    res_qp = run_gui.CursesGUI._parse_line_for_ui(None, line_qp)
    assert res_qp["type"] == "qp_found"

    # Overflow
    line_of = "overflow: threshold exceeded"
    res_of = run_gui.CursesGUI._parse_line_for_ui(None, line_of)
    assert res_of["type"] == "overflow"

    # Unstructured fallback
    line_unstructured = "random telemetry log message"
    res_unstructured = run_gui.CursesGUI._parse_line_for_ui(None, line_unstructured)
    assert res_unstructured["type"] == "unstructured"
    assert res_unstructured["line"] == "random telemetry log message"


# ==============================================================================
# 4. Headless Pipeline Integration Tests
# ==============================================================================


def test_curses_gui_headless_queue_processing(monkeypatch, tmp_path):
    """Test CursesGUI state updates and queue processing in headless mode."""
    # Prevent threads and draw loop from blocking
    monkeypatch.setattr(run_gui.CursesGUI, "_run_pipeline", lambda self: None)
    monkeypatch.setattr(run_gui.CursesGUI, "_draw_loop", lambda self: None)

    # Change directory so telemetry.json writes into tmp_path
    monkeypatch.chdir(tmp_path)

    monkeypatch.setattr(
        sys, "argv", ["run_gui.py", "--headless", "--min", "35", "--max", "37"]
    )
    args = run_gui.parse_args()

    gui = run_gui.CursesGUI(stdscr=None, args=args)

    # Inject messages into queue in logical pipeline execution order
    gui.queue.put({"type": "init"})
    gui.queue.put({"type": "lean_build", "sub": "START"})
    gui.queue.put({"type": "lean_build", "sub": "OK"})
    gui.queue.put({"type": "lean_log", "text": "Building module..."})
    gui.queue.put({"type": "lean_scan", "sorry": 0, "axiom": 0})
    gui.queue.put({"type": "target_bound", "min": "35", "max": "37"})
    gui.queue.put({"type": "retained_pruned", "retained": "120", "pruned": "880"})
    gui.queue.put(
        {
            "type": "json_progress",
            "event": {"Phase": {"phase": 2, "name": "DFS Construction"}},
        }
    )
    gui.queue.put(
        {
            "type": "json_progress",
            "event": {
                "StatusUpdate": {
                    "c": 50,
                    "total_weight_scaled": 100,
                    "comp": 50,
                    "active_str": "3, 5, 7",
                    "prefixes": 5,
                    "ap": 2,
                }
            },
        }
    )
    gui.queue.put({"type": "dfs_complete", "ab_pruned": 15, "z3": 8, "conflicts": 3})
    gui.queue.put({"type": "overflow"})
    gui.queue.put({"type": "qp_found", "line": "QUASIPERFECT NUMBER FOUND N=100"})
    gui.queue.put(
        {
            "type": "json_progress",
            "event": {
                "Done": {
                    "target_min_log10": 35,
                    "target_max_log10": 37,
                    "elapsed_ms": 50,
                }
            },
        }
    )
    gui.queue.put({"type": "success_exit"})

    gui._process_queue()

    assert gui.lean_initialized is True
    assert gui.target_bound == "10^35 < N < 10^37"
    assert gui.retained_comps == "120"
    assert gui.pruned_comps == "880"
    assert gui.abundance_pruned == 15
    assert gui.overflow_count == 1
    assert gui.qp_found == 1
    assert gui.phase_num == "4"
    assert gui.finished is True

    # Test export telemetry
    gui._export_telemetry()
    telemetry_file = tmp_path / "telemetry.json"
    assert telemetry_file.exists()
    telemetry_data = json.loads(telemetry_file.read_text(encoding="utf-8"))
    assert "progress_percentage" in telemetry_data
    assert "target_bounds" in telemetry_data
    assert "lean_statuses" in telemetry_data


def test_curses_gui_headless_pipeline_auto_raise(monkeypatch, tmp_path):
    """Test headless pipeline execution with mock subprocesses and auto-raising bounds."""
    monkeypatch.chdir(tmp_path)

    runs_count = [0]

    def mock_run_engine(self, release):
        runs_count[0] += 1
        if runs_count[0] == 1:
            # First run for 35..37 succeeds
            self.queue.put(
                {
                    "type": "json_progress",
                    "event": {
                        "Done": {
                            "target_min_log10": self.bound_min,
                            "target_max_log10": self.bound_max,
                            "elapsed_ms": 20,
                        }
                    },
                }
            )
            self.queue.put({"type": "success_exit"})
            return True
        else:
            # Second run auto-raised to 37..39, terminate loop
            self.queue.put(
                {
                    "type": "json_progress",
                    "event": {
                        "Done": {
                            "target_min_log10": self.bound_min,
                            "target_max_log10": self.bound_max,
                            "elapsed_ms": 20,
                        }
                    },
                }
            )
            self.queue.put({"type": "success_exit"})
            self.running = False
            return True

    def mock_run_lake_build(self, lean_project_dir, q):
        self.build_ok = True
        q.put({"type": "lean_build", "sub": "OK"})

    def mock_scan(self):
        self.sorry_count = 0
        self.axiom_count = 0
        self.theorems = {"qpn_is_odd_square": "verified"}

    def mock_draw_loop(self):
        # In headless mode, process queue and wait until running becomes False
        while self.running:
            self._process_queue()

    monkeypatch.setattr(run_gui.CursesGUI, "_run_engine", mock_run_engine)
    monkeypatch.setattr(run_gui.CursesGUI, "_draw_loop", mock_draw_loop)
    monkeypatch.setattr(run_gui.LeanProofStatus, "run_lake_build", mock_run_lake_build)
    monkeypatch.setattr(run_gui.LeanProofStatus, "scan", mock_scan)

    monkeypatch.setattr(
        sys,
        "argv",
        [
            "run_gui.py",
            "--headless",
            "--min",
            "35",
            "--max",
            "37",
            "--auto-raise",
            "--raise-step",
            "2",
        ],
    )
    args = run_gui.parse_args()

    gui = run_gui.CursesGUI(stdscr=None, args=args)

    # After initialization, pipeline should have executed run 1 (35..37),
    # auto-raised to 37..39, executed run 2 (37..39), and terminated.
    assert runs_count[0] == 2
    assert len(gui.bound_history) == 2
    assert gui.bound_history[0][0] == 35
    assert gui.bound_history[0][1] == 37
    assert gui.bound_history[1][0] == 37
    assert gui.bound_history[1][1] == 39
