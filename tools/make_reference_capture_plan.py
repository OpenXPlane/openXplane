#!/usr/bin/env python3
"""Prepare a native X-Plane QA screenshot plan; do not launch or modify the game.

The plan uses the documented QA command interface, not OS input injection.
Generated files, full Evidence, logs and original screenshots must stay local.
"""
import argparse
import datetime
import hashlib
import json
import math
import os
from pathlib import Path

REFERENCE_SHA256 = "2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def prepare(root, output, replay, interval, count):
    root = root.resolve(strict=True)
    exe = root / "X-Plane.exe"
    if digest(exe) != REFERENCE_SHA256:
        raise ValueError("reference executable SHA256 differs from the researched build")
    relative = Path(replay)
    if relative.is_absolute() or ".." in relative.parts:
        raise ValueError("replay must be a relative path inside the reference installation")
    replay_path = (root / relative).resolve(strict=True)
    if not replay_path.is_relative_to(root) or not replay_path.is_file():
        raise ValueError("replay must resolve to a file inside the reference installation")
    if not math.isfinite(interval) or interval <= 0 or not 1 <= count <= 100:
        raise ValueError("interval must be positive and finite; count must be 1..100")
    output = output.resolve()
    # The reference QA loader rejects drive-qualified paths (xmap.cpp:207),
    # even with forward slashes. Keep evidence outside the game and pass a
    # relative path from its working directory on the same drive.
    script = output / "capture.qa"
    try:
        qa_argument = Path(os.path.relpath(script, root)).as_posix()
    except ValueError as error:
        raise ValueError("QA evidence directory must be on the reference installation's drive") from error
    # Do not silently replace an earlier capture/evidence cohort.
    output.mkdir(parents=True, exist_ok=False)
    script.write_text(
        "# Native QA screenshot requests. Command success requires actual image evidence.\n"
        + (f"WAIT {interval:g}\nCMND sim/operation/screenshot\nNEXT_FRAME\n" * count),
        encoding="ascii",
    )
    args = [
        "--fps_test=1", "--require_fps=1",
        f"--load_smo={relative.as_posix()}", f"--qa_script={qa_argument}",
        "--safe_mode=PLG", "--no_sound", "--no_joysticks",
        "--window=1280x800", "--disable_networking",
    ]
    before = []
    image_dir = root / "Output"
    if image_dir.is_dir():
        for path in sorted(image_dir.rglob("*")):
            if path.is_file() and path.suffix.lower() in {".png", ".jpg", ".jpeg", ".bmp", ".tga"}:
                before.append({"path": path.relative_to(root).as_posix(),
                               "bytes": path.stat().st_size, "sha256": digest(path)})
    manifest = {
        "format": "openxplane-reference-qa-plan-v1",
        "prepared_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "executable": str(exe), "executable_sha256": REFERENCE_SHA256,
        "working_directory": str(root), "args": args,
        "replay": relative.as_posix(), "replay_sha256": digest(replay_path),
        "qa_script": str(script), "qa_sha256": digest(script),
        "requested_window": [1280, 800], "request_count": count,
        "wait_seconds_between_requests": interval, "output_images_before": before,
        "execution_status": "not_launched", "capture_status": "unproven",
        "limitations": [
            "A launch or QA request is not proof of a saved screenshot.",
            "Verify actual process identity/arguments, runtime log and new image files.",
            "Window size is requested, not measured. Replay does not fix all runtime state.",
            "Original and port require matching aircraft/scene/camera/time before pixel comparison.",
            "The reference may write logs, caches and screenshots during normal operation.",
            "Never relaunch a live target solely because an observation timed out.",
            "QA completion may leave the game running; inspect its actual state before cleanup.",
        ],
        "qa_documentation": "https://www.x-plane.com/kb/frame-rate-test/",
    }
    (output / "plan.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path)
    parser.add_argument("output", type=Path, help="new local evidence directory")
    parser.add_argument("--replay", default="Output/replays/fps_test_kpdx.fps")
    parser.add_argument("--interval", type=float, default=5)
    parser.add_argument("--count", type=int, default=3)
    args = parser.parse_args()
    plan = prepare(args.root, args.output, args.replay, args.interval, args.count)
    print(plan["qa_script"])
    print("Prepared only; execution and new image evidence remain unproven.")


if __name__ == "__main__":
    main()
