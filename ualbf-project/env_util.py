"""
UALBF Build Environment Variable Utility Module
================================================

Provides centralized, typed access to build environment variables registered in
`env_manifest.json`, along with runtime validation against `env_manifest.schema.json`
and immediate halting on deprecated bypass flags.
"""

import json
import os
import re
import sys
from pathlib import Path
from typing import Any, Dict, Optional

_MANIFEST_CACHE: Optional[Dict[str, Any]] = None
_SCHEMA_CACHE: Optional[Dict[str, Any]] = None


def find_repo_root(start_dir: Optional[Path] = None) -> Path:
    """Finds the repository root containing env_manifest.json or docs_manifest.json or proof_manifest.json."""
    current = Path(start_dir or Path(__file__)).resolve()
    if current.is_file():
        current = current.parent

    for p in [current] + list(current.parents):
        if (p / "env_manifest.json").exists():
            return p

    search_dirs = [current] + list(current.parents)
    try:
        cwd = Path.cwd().resolve()
        if cwd not in search_dirs:
            search_dirs.extend([cwd] + list(cwd.parents))
    except Exception:
        pass

    for p in search_dirs:
        if (
            (p / "env_manifest.json").exists()
            or (p / "docs_manifest.json").exists()
            or (p / "proof_manifest.json").exists()
            or (p / "bounds_manifest.json").exists()
        ):
            return p

    return current


def load_manifest_and_schema(
    manifest_path: Optional[str] = None, schema_path: Optional[str] = None
) -> tuple[Dict[str, Any], Dict[str, Any]]:
    """Loads and caches env_manifest.json and env_manifest.schema.json."""
    global _MANIFEST_CACHE, _SCHEMA_CACHE

    if _MANIFEST_CACHE is not None and _SCHEMA_CACHE is not None and not manifest_path:
        return _MANIFEST_CACHE, _SCHEMA_CACHE

    repo_root = find_repo_root()
    script_parent = Path(__file__).resolve().parent

    if not manifest_path:
        possible_manifests = [
            repo_root / "env_manifest.json",
            Path(__file__).resolve().parent / "env_manifest.json",
            repo_root / "ualbf-project" / "env_manifest.json",
            Path(__file__).resolve().parent / "env_manifest.json",
            Path(__file__).resolve().parent.parent / "env_manifest.json",
            repo_root.parent / "env_manifest.json",
            script_parent / "env_manifest.json",
            Path.cwd() / "env_manifest.json",
            Path.cwd().parent / "env_manifest.json",
            Path(__file__).resolve().parent / "env_manifest.json",
            Path(__file__).resolve().parent.parent / "env_manifest.json",
        ]
        try:
            possible_manifests.extend(
                [
                    Path.cwd() / "env_manifest.json",
                    Path.cwd().parent / "env_manifest.json",
                ]
            )
        except Exception:
            pass

        m_path = next((p for p in possible_manifests if p.exists()), None)
        if not m_path:
            raise FileNotFoundError("env_manifest.json not found in repository root.")
    else:
        m_path = Path(manifest_path)

    if not schema_path:
        possible_schemas = [
            repo_root / "env_manifest.schema.json",
            Path(__file__).resolve().parent / "env_manifest.schema.json",
            repo_root / "ualbf-project" / "env_manifest.schema.json",
            Path(__file__).resolve().parent / "env_manifest.schema.json",
            Path(__file__).resolve().parent.parent / "env_manifest.schema.json",
            repo_root.parent / "env_manifest.schema.json",
            script_parent / "env_manifest.schema.json",
            Path.cwd() / "env_manifest.schema.json",
            Path.cwd().parent / "env_manifest.schema.json",
            Path(__file__).resolve().parent / "env_manifest.schema.json",
            Path(__file__).resolve().parent.parent / "env_manifest.schema.json",
        ]
        try:
            possible_schemas.extend(
                [
                    Path.cwd() / "env_manifest.schema.json",
                    Path.cwd().parent / "env_manifest.schema.json",
                ]
            )
        except Exception:
            pass

        s_path = next((p for p in possible_schemas if p.exists()), None)
        if not s_path:
            raise FileNotFoundError(
                "env_manifest.schema.json not found in repository root."
            )
    else:
        s_path = Path(schema_path)

    with open(m_path, "r", encoding="utf-8") as f:
        manifest = json.load(f)

    with open(s_path, "r", encoding="utf-8") as f:
        schema = json.load(f)

    validate_manifest_schema(manifest, schema)

    _MANIFEST_CACHE = manifest
    _SCHEMA_CACHE = schema
    return manifest, schema


def validate_manifest_schema(manifest: Dict[str, Any], schema: Dict[str, Any]) -> None:
    """Validates the structure of env_manifest.json against env_manifest.schema.json rules."""
    if not isinstance(manifest, dict):
        raise ValueError("env_manifest.json root must be a JSON object.")

    valid_types = {"string", "integer", "float", "boolean", "path"}
    valid_statuses = {"active", "deprecated", "internal"}

    for key, val in manifest.items():
        if not re.match(r"^[A-Z0-9_]+$", key):
            raise ValueError(f"Invalid environment variable name in manifest: '{key}'")

        if not isinstance(val, dict):
            raise ValueError(f"Manifest entry for '{key}' must be an object.")

        for req in ["type", "default", "status", "description"]:
            if req not in val:
                raise ValueError(
                    f"Manifest entry for '{key}' missing required property '{req}'."
                )

        if val["type"] not in valid_types:
            raise ValueError(
                f"Manifest entry for '{key}' has invalid type '{val['type']}'."
            )

        if val["status"] not in valid_statuses:
            raise ValueError(
                f"Manifest entry for '{key}' has invalid status '{val['status']}'."
            )

        if not isinstance(val["description"], str):
            raise ValueError(
                f"Manifest entry for '{key}' description must be a string."
            )

        allowed = val.get("allowed_values")
        if allowed is not None and not isinstance(allowed, (list, tuple)):
            raise ValueError(
                f"Manifest entry for '{key}' allowed_values must be a list or null."
            )


def check_deprecated_env_vars() -> None:
    """Checks for deprecated environment variables in os.environ and halts execution if set."""
    manifest, _ = load_manifest_and_schema()

    for key, details in manifest.items():
        if details.get("status") == "deprecated":
            if key in os.environ:
                print(
                    f"Error: Bypass options are deprecated and verification cannot be skipped. "
                    f"Found deprecated flag '{key}'."
                )
                sys.exit(1)


def get_env_var(name: str, default: Any = None) -> Any:
    """
    Retrieves and validates a typed environment variable registered in env_manifest.json.
    Halts execution if any deprecated flag is set.
    """
    check_deprecated_env_vars()
    manifest, _ = load_manifest_and_schema()

    if name not in manifest:
        raise ValueError(
            f"Environment variable '{name}' is not registered in env_manifest.json."
        )

    meta = manifest[name]
    var_type = meta["type"]
    allowed_values = meta.get("allowed_values")

    if name in os.environ:
        raw_val = os.environ[name]

        if allowed_values is not None:
            if str(raw_val) not in [str(av) for av in allowed_values]:
                raise ValueError(
                    f"Environment variable '{name}' value '{raw_val}' not in allowed values: {allowed_values}"
                )

        if var_type == "boolean":
            lower = str(raw_val).strip().lower()
            if lower in ("1", "true", "yes", "y", "on"):
                return True
            elif lower in ("0", "false", "no", "n", "off", ""):
                return False
            else:
                raise ValueError(
                    f"Invalid boolean value for environment variable '{name}': '{raw_val}'"
                )
        elif var_type == "integer":
            try:
                return int(raw_val)
            except ValueError:
                raise ValueError(
                    f"Invalid integer value for environment variable '{name}': '{raw_val}'"
                )
        elif var_type == "float":
            try:
                return float(raw_val)
            except ValueError:
                raise ValueError(
                    f"Invalid float value for environment variable '{name}': '{raw_val}'"
                )
        else:
            return raw_val

    if default is not None:
        return default

    return meta.get("default")


def require_env_var(name: str) -> Any:
    """
    Retrieves a required environment variable registered in env_manifest.json.
    Raises ValueError if the variable is missing or empty.
    """
    check_deprecated_env_vars()
    if name not in os.environ or not str(os.environ[name]).strip():
        print(
            f"Error: Required environment variable '{name}' is not set.",
            file=sys.stderr,
        )
        raise ValueError(f"Required environment variable '{name}' is not set.")
    return get_env_var(name)


def validate_environment() -> None:
    """Validates active environment variables against env_manifest.json specifications."""
    check_deprecated_env_vars()
    manifest, _ = load_manifest_and_schema()
    for name in os.environ:
        if name in manifest:
            get_env_var(name)
