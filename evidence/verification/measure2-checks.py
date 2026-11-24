"""Focused metric arithmetic and PID-identity cleanup checks; no GUI verdict."""

import subprocess
import sys
import unittest
from pathlib import Path

import psutil

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
from measure_app_idle import cleanup, summary


class SamplerChecks(unittest.TestCase):
    # Value: protects=all five current summary() distributions
    # fails_when=final working-set or process-count output is removed or mis-mapped
    # why_new=previous synthetic test asserted only private/peak distributions
    # seam=none
    # no_claim=no fresh measurement, GUI rerun, or performance verdict
    def test_summary_retains_run_range_and_sampled_peaks(self):
        rows = [
            {
                "final": {
                    "private_commit_bytes": a,
                    "working_set_bytes": b,
                    "processes": a // 10,
                },
                "sampled_peak_private_bytes": c,
                "sampled_peak_working_set_bytes": d,
            }
            for a, b, c, d in [(10, 20, 30, 40), (20, 40, 90, 110), (30, 60, 50, 80)]
        ]
        result = summary(rows)
        self.assertEqual(
            result["final_private_bytes"], {"median": 20, "min": 10, "max": 30}
        )
        self.assertEqual(
            result["final_working_set_bytes"], {"median": 40, "min": 20, "max": 60}
        )
        self.assertEqual(
            result["final_process_count"], {"median": 2, "min": 1, "max": 3}
        )
        self.assertEqual(
            result["sampled_peak_private_bytes"], {"median": 50, "min": 30, "max": 90}
        )
        self.assertEqual(
            result["sampled_peak_working_set_bytes"],
            {"median": 80, "min": 40, "max": 110},
        )

    def test_cleanup_preserves_changed_birth_identity_and_reaps_owned_root(self):
        child = subprocess.Popen(
            [sys.executable, "-c", "import time; time.sleep(30)"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        process = psutil.Process(child.pid)
        born = process.create_time()
        try:
            cleanup(None, {child.pid: born + 1})
            self.assertIsNone(child.poll(), "changed identity must not be killed")
            cleanup(child, {child.pid: born})
            self.assertIsNotNone(child.poll(), "launch-owned root must be reaped")
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
