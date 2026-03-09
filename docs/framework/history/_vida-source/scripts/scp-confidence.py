#!/usr/bin/env python3
import argparse
import json


WEIGHTS = {
    "user_alignment": 0.25,
    "api_reality": 0.25,
    "evidence_quality": 0.20,
    "architecture_fit": 0.15,
    "delivery_readiness": 0.15,
}


def clamp(v: float) -> float:
    return max(0.0, min(100.0, v))


def band(score: float) -> str:
    if score >= 85:
        return "ready"
    if score >= 70:
        return "conditional"
    return "not_ready"


def compute(values):
    weighted = 0.0
    parts = {}
    for k, w in WEIGHTS.items():
        v = clamp(float(values[k]))
        parts[k] = {"value": v, "weight": w, "contribution": round(v * w, 2)}
        weighted += v * w
    score = round(weighted, 2)
    return {
        "score": score,
        "band": band(score),
        "parts": parts,
    }


def main():
    p = argparse.ArgumentParser(description="SCP confidence score calculator")
    p.add_argument("--user-alignment", type=float, required=True)
    p.add_argument("--api-reality", type=float, required=True)
    p.add_argument("--evidence-quality", type=float, required=True)
    p.add_argument("--architecture-fit", type=float, required=True)
    p.add_argument("--delivery-readiness", type=float, required=True)
    p.add_argument("--json", action="store_true")
    args = p.parse_args()

    values = {
        "user_alignment": args.user_alignment,
        "api_reality": args.api_reality,
        "evidence_quality": args.evidence_quality,
        "architecture_fit": args.architecture_fit,
        "delivery_readiness": args.delivery_readiness,
    }
    out = compute(values)

    if args.json:
        print(json.dumps(out, ensure_ascii=False, indent=2))
        return

    print(f"SCP confidence: {out['score']} ({out['band']})")
    for k in [
        "user_alignment",
        "api_reality",
        "evidence_quality",
        "architecture_fit",
        "delivery_readiness",
    ]:
        part = out["parts"][k]
        print(f"- {k}: {part['value']} * {part['weight']} = {part['contribution']}")


if __name__ == "__main__":
    main()

