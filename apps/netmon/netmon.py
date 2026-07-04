#!/usr/bin/env python3
"""Network connectivity monitor — periodic probing.

Probes network reachability at a configurable interval using ICMP ping
(fast-fail on Windows when disconnected) with TCP fallback.  Uses
**majority voting**: at least half the targets must be reachable for
the network to be considered UP.  When state changes, the event is
logged to console and a UTF-8 log file.  Press Ctrl+C to stop.

Pure Python stdlib.  Zero external dependencies.

Usage:
  python netmon.py                  # defaults: 1s interval, 3 targets
  python netmon.py -i 5             # probe every 5 seconds
  python netmon.py -t 8.8.8.8 -t 1.1.1.1  # custom targets
  python netmon.py -v               # verbose: show per-target results
  python netmon.py -q               # quiet: only status changes
"""

from __future__ import annotations

import argparse
import signal
import socket
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path
from typing import Optional

# ═══════════════════════════════════════════════════════════════════════════════
# Data
# ═══════════════════════════════════════════════════════════════════════════════

DEFAULT_TARGETS = [
    "8.8.8.8",       # Google DNS
    "1.1.1.1",       # Cloudflare DNS
    "baidu.com",     # domestic reachability
]

TCP_FALLBACK_PORT = 53


@dataclass
class ProbeResult:
    """Outcome of probing a single target."""
    host: str
    method: str       # "icmp" or "tcp"
    ok: bool
    detail: str


@dataclass
class OutageEvent:
    """A single network outage event."""
    start: datetime
    end: Optional[datetime] = None
    duration: Optional[timedelta] = None


@dataclass
class Summary:
    """Aggregate statistics over a monitoring session."""
    total_runtime: timedelta
    outage_count: int
    total_downtime: timedelta
    longest_outage: timedelta
    availability_pct: float


@dataclass
class Config:
    """Runtime configuration parsed from CLI args."""
    targets: list[str]
    log_path: Path
    timeout: float
    use_tcp_fallback: bool
    interval: float
    threshold: int        # min reachable targets to declare UP
    quiet: bool
    verbose: bool


# ═══════════════════════════════════════════════════════════════════════════════
# Prober  —  probes every target, returns per-target results + verdict
# ═══════════════════════════════════════════════════════════════════════════════

class Prober:
    """Probes all targets **in parallel** and applies majority-vote decision.

    ICMP pings to all targets are issued simultaneously via a thread pool,
    so a single slow / timing-out host doesn't delay the others.  TCP
    fallback runs as a second parallel wave for hosts where ICMP failed.

    The overall verdict is UP when at least *threshold* targets are reachable.
    """

    def __init__(
        self,
        targets: list[str],
        timeout: float,
        use_tcp_fallback: bool,
        threshold: int,
    ) -> None:
        self._targets = targets
        self._timeout = timeout
        self._use_tcp = use_tcp_fallback
        self._threshold = threshold

    def probe(self) -> tuple[bool, list[ProbeResult]]:
        """Return (reachable, per_target_results)."""
        n = len(self._targets)

        # ── Phase 1: parallel ICMP ───────────────────────────────────
        results: dict[str, ProbeResult] = {}
        failed: list[str] = []

        with ThreadPoolExecutor(max_workers=n) as pool:
            futures = {
                pool.submit(self._icmp_probe, host): host
                for host in self._targets
            }
            for f in as_completed(futures):
                host = futures[f]
                ok, info = f.result()
                if ok:
                    results[host] = ProbeResult(host, "icmp", True, info)
                else:
                    failed.append(host)
                    results[host] = ProbeResult(host, "icmp", False, info)

        # Early exit: threshold already met
        up_count = sum(1 for r in results.values() if r.ok)
        if up_count >= self._threshold:
            return True, [results[h] for h in self._targets]

        # ── Phase 2: parallel TCP fallback for failed hosts ──────────
        if self._use_tcp and failed:
            with ThreadPoolExecutor(max_workers=len(failed)) as pool:
                futures = {
                    pool.submit(self._tcp_probe, host, TCP_FALLBACK_PORT): host
                    for host in failed
                }
                for f in as_completed(futures):
                    host = futures[f]
                    ok, info = f.result()
                    if ok:
                        results[host] = ProbeResult(host, "tcp", True, info)

        up_count = sum(1 for r in results.values() if r.ok)
        reachable = up_count >= self._threshold
        return reachable, [results[h] for h in self._targets]

    def _run(self, cmd: list[str], timeout: float) -> subprocess.CompletedProcess:
        try:
            return subprocess.run(
                cmd,
                capture_output=True,
                timeout=timeout + 1,
            )
        except subprocess.TimeoutExpired:
            return subprocess.CompletedProcess(
                cmd, returncode=-1, stdout=b"", stderr=b"timed out"
            )

    def _icmp_probe(self, host: str) -> tuple[bool, str]:
        timeout_ms = int(self._timeout * 1000)
        if sys.platform == "win32":
            cmd = ["ping", "-n", "1", "-w", str(timeout_ms), host]
        else:
            cmd = ["ping", "-c", "1", "-W", str(int(self._timeout)), host]
        result = self._run(cmd, self._timeout)
        ok = result.returncode == 0
        return ok, f"{host}: {'ok' if ok else 'fail'}"

    def _tcp_probe(self, host: str, port: int) -> tuple[bool, str]:
        try:
            sock = socket.create_connection((host, port), timeout=self._timeout)
            sock.close()
            return True, f"{host}:{port}: ok"
        except (socket.timeout, OSError) as exc:
            reason = _exc_summary(exc)
            return False, f"{host}:{port}: {reason}"


def _exc_summary(exc: OSError) -> str:
    try:
        return exc.strerror or type(exc).__name__
    except Exception:
        return type(exc).__name__


# ═══════════════════════════════════════════════════════════════════════════════
# OutageTracker
# ═══════════════════════════════════════════════════════════════════════════════

class OutageTracker:
    """Tracks network outage events and produces aggregate statistics."""

    def __init__(self) -> None:
        self._outages: list[OutageEvent] = []
        self._current: Optional[OutageEvent] = None
        self._start_time: Optional[datetime] = None

    @property
    def is_down(self) -> bool:
        return self._current is not None

    def start(self) -> None:
        self._start_time = datetime.now()

    def record_down(self, dt: datetime) -> None:
        if self._current is None:
            self._current = OutageEvent(start=dt)

    def record_up(self, dt: datetime) -> None:
        if self._current is not None:
            self._current.end = dt
            self._current.duration = dt - self._current.start
            self._outages.append(self._current)
            self._current = None

    @property
    def last_outage(self) -> Optional[OutageEvent]:
        return self._outages[-1] if self._outages else None

    def summary(self) -> Summary:
        now = datetime.now()
        runtime = (now - self._start_time) if self._start_time else timedelta()

        all_outages = list(self._outages)
        if self._current is not None:
            all_outages.append(OutageEvent(
                start=self._current.start,
                end=now,
                duration=now - self._current.start,
            ))

        count = len(all_outages)
        total_down = sum(
            (o.duration for o in all_outages if o.duration is not None),
            timedelta(),
        )
        longest = max(
            (o.duration for o in all_outages if o.duration is not None),
            default=timedelta(),
        )

        if runtime.total_seconds() > 0:
            avail = (
                (runtime.total_seconds() - total_down.total_seconds())
                / runtime.total_seconds()
                * 100
            )
        else:
            avail = 100.0

        return Summary(
            total_runtime=runtime,
            outage_count=count,
            total_downtime=total_down,
            longest_outage=longest,
            availability_pct=avail,
        )


# ═══════════════════════════════════════════════════════════════════════════════
# Logger
# ═══════════════════════════════════════════════════════════════════════════════

class Logger:
    """Dual-output logger (console + UTF-8 file), Windows-encoding-safe."""

    def __init__(self, log_path: Path, quiet: bool = False) -> None:
        self._quiet = quiet
        self._file = open(str(log_path), "a", encoding="utf-8")
        self._console_enc = sys.stdout.encoding or "utf-8"

    def info(self, msg: str) -> None:
        self._write("INFO ", "+", msg)

    def warn(self, msg: str) -> None:
        self._write("WARN ", "!", msg)

    def debug(self, msg: str) -> None:
        now = datetime.now()
        self._file.write(
            f"[{now.strftime('%Y-%m-%d %H:%M:%S')}] DEBUG {msg}\n"
        )
        self._file.flush()
        self._print_console(f"[{now.strftime('%H:%M:%S')}] ~ {msg}")

    def raw(self, text: str) -> None:
        self._file.write(text + "\n")
        self._file.flush()
        self._print_console(text)

    def _write(self, level: str, prefix: str, msg: str) -> None:
        now = datetime.now()
        self._file.write(
            f"[{now.strftime('%Y-%m-%d %H:%M:%S')}] {level}{msg}\n"
        )
        self._file.flush()

        if self._quiet and level.strip() == "INFO":
            return

        self._print_console(f"[{now.strftime('%H:%M:%S')}] {prefix} {msg}")

    def _print_console(self, text: str) -> None:
        try:
            print(text, flush=True)
        except UnicodeEncodeError:
            safe = text.encode(self._console_enc, errors="replace").decode(
                self._console_enc
            )
            print(safe, flush=True)

    def close(self) -> None:
        self._file.close()


# ═══════════════════════════════════════════════════════════════════════════════
# Helpers
# ═══════════════════════════════════════════════════════════════════════════════

def _fmt_duration(d: timedelta) -> str:
    total = int(d.total_seconds())
    if total < 60:
        return f"{total}s"
    if total < 3600:
        m, s = divmod(total, 60)
        return f"{m}m {s}s"
    h, rem = divmod(total, 3600)
    m, s = divmod(rem, 60)
    return f"{h}h {m}m {s}s"


def _fmt_results(results: list[ProbeResult]) -> str:
    """Format per-target probe results into a compact line.

    Example:  "8.8.8.8✓ 1.1.1.1✓ baidu.com✓"
              "8.8.8.8✗ 1.1.1.1✗ baidu.com✓  ← only baidu"
    """
    parts = []
    for r in results:
        mark = "v" if r.ok else "x"
        parts.append(f"{r.host}{mark}")
    return " ".join(parts)


# ═══════════════════════════════════════════════════════════════════════════════
# Monitor
# ═══════════════════════════════════════════════════════════════════════════════

class Monitor:
    """Polling-based network connectivity monitor with majority voting.

    Probes all targets every *interval* seconds.  At least *threshold*
    targets must be UP for the network to be considered UP.
    """

    def __init__(self, config: Config) -> None:
        self._config = config
        self._prober = Prober(
            config.targets,
            config.timeout,
            config.use_tcp_fallback,
            config.threshold,
        )
        self._tracker = OutageTracker()
        self._logger = Logger(config.log_path, config.quiet)
        self._is_down: bool = False
        self._running = False

    def run(self) -> None:
        targets_str = ", ".join(self._config.targets)
        tcp_note = "+tcp" if self._config.use_tcp_fallback else "icmp-only"
        self._logger.info(
            f"Monitor started (targets={targets_str}, "
            f"interval={self._config.interval}s, threshold={self._config.threshold}/{len(self._config.targets)}, mode={tcp_note})"
        )

        # ── initial probe ───────────────────────────────────────────────
        self._tracker.start()
        self._do_probe(initial=True)

        # ── signal handler ──────────────────────────────────────────────
        self._running = True

        def _on_sigint(_signum: int, _frame: object) -> None:
            self._running = False

        original = signal.signal(signal.SIGINT, _on_sigint)

        try:
            self._main_loop()
        finally:
            signal.signal(signal.SIGINT, original)
            if self._tracker.is_down:
                self._tracker.record_up(datetime.now())
            self._print_summary()
            self._logger.close()

    # ── internals ───────────────────────────────────────────────────────

    def _main_loop(self) -> None:
        while self._running:
            time.sleep(self._config.interval)
            if not self._running:
                break
            self._do_probe()

    def _do_probe(self, initial: bool = False) -> None:
        ok, results = self._prober.probe()
        now = datetime.now()

        # ── verbose: always show per-target status ──────────────────────
        if self._config.verbose:
            status = "UP" if ok else "DOWN"
            self._logger.debug(f"[{status}] {_fmt_results(results)}")

        # ── state transitions ───────────────────────────────────────────
        if ok and self._is_down:
            self._is_down = False
            self._tracker.record_up(now)
            last = self._tracker.last_outage
            duration = ""
            if last is not None and last.duration is not None:
                duration = f" (was down for {_fmt_duration(last.duration)})"
            self._logger.info(f"Network UP{duration}")

        elif not ok and not self._is_down:
            self._is_down = True
            self._tracker.record_down(now)
            down_hosts = [r.host for r in results if not r.ok]
            detail = f" ({', '.join(down_hosts)} unreachable)" if down_hosts else ""
            tag = "initial check" if initial else ""
            msg = f"Network DOWN{detail}"
            if tag:
                msg += f" ({tag})"
            self._logger.warn(msg)

    def _print_summary(self) -> None:
        s = self._tracker.summary()
        lines = [
            "=== Summary ===",
            f"  Total runtime:     {_fmt_duration(s.total_runtime)}",
            f"  Total outages:     {s.outage_count}",
            f"  Total downtime:    {_fmt_duration(s.total_downtime)}",
            f"  Longest outage:    {_fmt_duration(s.longest_outage)}",
            f"  Availability:      {s.availability_pct:.2f}%",
            "===============",
        ]
        for line in lines:
            self._logger.raw(line)


# ═══════════════════════════════════════════════════════════════════════════════
# CLI
# ═══════════════════════════════════════════════════════════════════════════════

def parse_args(argv: Optional[list[str]] = None) -> Config:
    p = argparse.ArgumentParser(
        description="Network connectivity monitor (majority-vote probing).",
    )
    p.add_argument(
        "-i", "--interval",
        type=float,
        default=1.0,
        help="Probe interval in seconds (default: 1)",
    )
    p.add_argument(
        "-t", "--targets",
        dest="targets",
        action="append",
        default=None,
        metavar="HOST",
        help="Probe target host; repeat for multiple (default: 8.8.8.8 1.1.1.1 baidu.com)",
    )
    p.add_argument(
        "--log",
        type=Path,
        default=Path("netmon.log"),
        help="Log file path (default: ./netmon.log)",
    )
    p.add_argument(
        "--timeout",
        type=float,
        default=2.0,
        help="Single-probe timeout in seconds (default: 2)",
    )
    p.add_argument(
        "--threshold",
        type=int,
        default=None,
        metavar="N",
        help="Min reachable targets to declare UP (default: ceil(targets/2), i.e. majority)",
    )
    p.add_argument(
        "--no-tcp-fallback",
        action="store_true",
        help="Disable TCP fallback (ICMP-only mode)",
    )
    p.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Show per-target results for every probe",
    )
    p.add_argument(
        "-q", "--quiet",
        action="store_true",
        help="Only print status changes (DOWN / UP)",
    )

    args = p.parse_args(argv)

    targets = args.targets if args.targets else list(DEFAULT_TARGETS)
    threshold = args.threshold if args.threshold is not None else (len(targets) // 2 + 1)

    if threshold > len(targets):
        p.error(f"--threshold ({threshold}) cannot exceed target count ({len(targets)})")
    if threshold < 1:
        p.error(f"--threshold must be >= 1, got {threshold}")

    return Config(
        targets=targets,
        log_path=args.log,
        timeout=args.timeout,
        use_tcp_fallback=not args.no_tcp_fallback,
        interval=args.interval,
        threshold=threshold,
        quiet=args.quiet,
        verbose=args.verbose,
    )


# ═══════════════════════════════════════════════════════════════════════════════
# Entry point
# ═══════════════════════════════════════════════════════════════════════════════

def main() -> None:
    config = parse_args()
    monitor = Monitor(config)
    monitor.run()


if __name__ == "__main__":
    main()
