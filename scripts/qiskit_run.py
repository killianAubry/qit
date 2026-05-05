"""qsim_ui Qiskit runner.

Usage:
    python3 qiskit_run.py <mode> <file>
    mode = qasm | py

QASM: <file> is OpenQASM 2 → QuantumCircuit.from_qasm_str
PY:   <file> is Python → exec'd; finds `qc` or `circuit` (or last QuantumCircuit
      defined in the module namespace).

Output (stdout): JSON {num_qubits, probabilities, amplitudes, bloch}.
Errors: human-readable text to stderr; non-zero exit code.

Requires: pip install qiskit
"""

from __future__ import annotations

import json
import sys
import traceback
from pathlib import Path


def _load_qiskit():
    try:
        from qiskit import QuantumCircuit  # type: ignore
        from qiskit.quantum_info import Statevector  # type: ignore
    except ImportError as e:
        print(
            f"missing qiskit: {e}\n"
            f"install with:  python3 -m pip install qiskit",
            file=sys.stderr,
        )
        sys.exit(66)
    return QuantumCircuit, Statevector


def _build_from_qasm(QuantumCircuit, src):
    try:
        return QuantumCircuit.from_qasm_str(src)
    except Exception as e:
        print(f"openqasm parse error:\n  {e}", file=sys.stderr)
        sys.exit(67)


def _build_from_python(QuantumCircuit, src, path):
    ns = {"__name__": "__qsim_ui__", "__file__": str(path)}
    try:
        code = compile(src, str(path), "exec")
        exec(code, ns)  # noqa: S102 — intentional sandbox-less exec
    except SystemExit:
        raise
    except Exception:
        tb = traceback.format_exc(limit=8)
        print(f"python error:\n{tb}", file=sys.stderr)
        sys.exit(67)

    # Prefer obvious names, then fall back to the last circuit produced.
    qc = ns.get("qc") or ns.get("circuit")
    if not isinstance(qc, QuantumCircuit):
        qc = None
        for v in reversed(list(ns.values())):
            if isinstance(v, QuantumCircuit):
                qc = v
                break
    if qc is None:
        print(
            "no QuantumCircuit found.\n"
            "  define one as `qc = QuantumCircuit(...)` (or `circuit = ...`).",
            file=sys.stderr,
        )
        sys.exit(68)
    return qc


def main() -> None:
    if len(sys.argv) < 3:
        print("usage: qiskit_run.py <qasm|py> <file>", file=sys.stderr)
        sys.exit(64)

    mode = sys.argv[1].strip().lower()
    path = Path(sys.argv[2])
    if not path.is_file():
        print(f"not a file: {path}", file=sys.stderr)
        sys.exit(65)

    src = path.read_text(encoding="utf-8")

    QuantumCircuit, Statevector = _load_qiskit()

    if mode == "qasm":
        qc = _build_from_qasm(QuantumCircuit, src)
    elif mode == "py":
        qc = _build_from_python(QuantumCircuit, src, path)
    else:
        print(f"unknown mode: {mode!r}; expected 'qasm' or 'py'", file=sys.stderr)
        sys.exit(64)

    nq = qc.num_qubits
    if nq <= 0:
        print("circuit has no qubits", file=sys.stderr)
        sys.exit(68)
    if nq > 10:
        print(f"too many qubits: {nq} (UI limit 10)", file=sys.stderr)
        sys.exit(69)

    # Drop final measurements / barriers for unitary evolution. The qsim_ui
    # caller derives probabilities + Bloch from the pre-measurement state.
    qc_ev = qc.copy()
    try:
        qc_ev.remove_final_measurements(inplace=True)
    except Exception:
        qc_ev = qc.copy()

    try:
        sv = Statevector(qc_ev)
    except Exception as e:
        print(f"statevector failed: {e}", file=sys.stderr)
        sys.exit(70)

    out = {
        "num_qubits": int(nq),
        "statevector": [
            {"re": float(z.real), "im": float(z.imag)} for z in sv.data
        ],
    }
    json.dump(out, sys.stdout)
    sys.stdout.write("\n")


if __name__ == "__main__":
    main()
