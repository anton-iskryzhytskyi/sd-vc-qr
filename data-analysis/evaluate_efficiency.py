
# Efficiency scores (Sign, Verify, Size, WNC, Geometric) for SD‑JWT VC
# do I need geometric???

import argparse
from pathlib import Path

import numpy as np
import pandas as pd


def _load_data(data_dir: Path):
    comb = pd.read_csv(data_dir / "combined_summary.csv")
    size = pd.read_csv(data_dir / "size_data.csv")

    if "jwt_size_bytes" not in size.columns:
        raise ValueError("size_data.csv must contain 'jwt_size_bytes'.")
    return comb, size


def _aggregate_latency(df: pd.DataFrame, op: str, agg: str = "median"):
    valid = df[df["operation"] == op]
    if agg == "median":
        return (
            valid.groupby("wallet_type")["mean_time_ms"]
            .median()
            .rename(f"T_{op}")
        )
    if agg == "mean":
        return (
            valid.groupby("wallet_type")["mean_time_ms"]
            .mean()
            .rename(f"T_{op}")
        )
    raise ValueError("agg must be 'median' or 'mean'")


def _normalise(col: pd.Series) -> pd.Series:
    return col / col.min()


def _merge_all(sign_s, verify_s, size_s):
    df = pd.concat([sign_s, verify_s, size_s], axis=1, join="inner")
    return df.rename(columns={"jwt_size_bytes": "Size_B"})


def _compute_scores(df: pd.DataFrame, w_sign, w_ver, w_size, kappa):
    df["T_sign_n"] = _normalise(df["T_issue"])
    df["T_verify_n"] = _normalise(df["T_verify"])
    df["Size_n"] = _normalise(df["Size_B"])

    df["WNC"] = (
        w_sign * df["T_sign_n"]
        + w_ver * df["T_verify_n"]
        + w_size * df["Size_n"]
    )

    df["Geom"] = (df["T_issue"] * df["T_verify"] * df["Size_B"] ** kappa) ** (
        1.0 / (2.0 + kappa)
    )
    return df


def _pretty_print(df: pd.DataFrame, title: str):
    print(f"\n{title}")
    print("=" * len(title))
    cols = ["T_issue", "T_verify", "Size_B", "WNC", "Geom"]
    print(df[cols].sort_values("WNC").to_string(float_format=lambda x: f"{x:,.3f}"))


def main():
    ap = argparse.ArgumentParser(
        description="Compute efficiency scores for SD‑JWT VC benchmarks"
    )
    ap.add_argument("--data-dir", default="../target/criterion")
    ap.add_argument("--agg", default="median", choices=["median", "mean"])
    ap.add_argument("--w-sign", type=float, default=0.2)
    ap.add_argument("--w-verify", type=float, default=0.5)
    ap.add_argument("--w-size", type=float, default=0.3)
    ap.add_argument("--kappa", type=float, default=1.0)
    ap.add_argument("--per-scenario", action="store_true")
    args = ap.parse_args()

    args.data_dir = Path(args.data_dir)

    comb, size = _load_data(args.data_dir)

    sign = _aggregate_latency(comb, "issue", args.agg).rename("T_issue")
    verify = _aggregate_latency(comb, "verify", args.agg).rename("T_verify")

    master = _merge_all(sign, verify, size.groupby("wallet_type")["jwt_size_bytes"].mean())
    master = _compute_scores(
        master, args.w_sign, args.w_verify, args.w_size, args.kappa
    )

    _pretty_print(master, "GLOBAL SCORES")

    if args.per_scenario:
        scenarios = (
            comb[["field_count", "field_size"]]
            .drop_duplicates()
            .sort_values(["field_count", "field_size"])
            .to_dict("records")
        )
        for sc in scenarios:
            fc, fs = sc["field_count"], sc["field_size"]
            mask = (comb["field_count"] == fc) & (comb["field_size"] == fs)

            s_sc = _aggregate_latency(comb[mask], "issue", args.agg).rename("T_issue")
            v_sc = _aggregate_latency(comb[mask], "verify", args.agg).rename("T_verify")
            m_sc = _merge_all(
                s_sc,
                v_sc,
                size.groupby("wallet_type")["jwt_size_bytes"].mean(),
            )
            m_sc = _compute_scores(
                m_sc, args.w_sign, args.w_verify, args.w_size, args.kappa
            )
            title = f"SCENARIO  fields={fc:>5}  field_size={fs:>4} B"
            _pretty_print(m_sc, title)


if __name__ == "__main__":
    main()

"""
 python evaluate_efficiency.py --data-dir ../raw-data/benchmark_data \
    --w-sign 0.2 --w-verify 0.5 --w-size 0.3
"""