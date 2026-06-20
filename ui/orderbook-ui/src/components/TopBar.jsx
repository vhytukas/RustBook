import { useEffect, useState } from "react";

const fmt = (ns) => {
  if (ns == null) return "—";
  if (ns < 1000) return `${ns} ns`;
  if (ns < 1_000_000) return `${(ns / 1000).toFixed(1)} µs`;
  return `${(ns / 1_000_000).toFixed(2)} ms`;
};

const TopBar = ({ githubUrl = "https://github.com/Vytas-Mar/RustBook" }) => {
  const [perf, setPerf] = useState(null);

  useEffect(() => {
    fetch("/bench-results.json")
      .then((r) => (r.ok ? r.json() : null))
      .then((data) => {
        if (!data?.benches?.length) return;
        const headline =
          data.benches.find((b) => b.name.startsWith("burst_n100k_lambda_1k")) ??
          data.benches[0];
        setPerf({
          p50: headline.p50_ns,
          p99: headline.p99_ns,
          p999: headline.p999_ns,
        });
      })
      .catch(() => {});
  }, []);

  return (
    <header className="hero">
      <div className="hero-brand">
        <div className="hero-name">RUSTBOOK</div>
        <div className="hero-tagline">
          Deterministic matching engine · Rust + WASM
        </div>
      </div>

      <div className="hero-perf">
        <div className="hero-perf-item">
          <span className="hero-perf-label">p50</span>
          <span className="hero-perf-value">{fmt(perf?.p50)}</span>
        </div>
        <div className="hero-perf-item">
          <span className="hero-perf-label">p99</span>
          <span className="hero-perf-value">{fmt(perf?.p99)}</span>
        </div>
        <div className="hero-perf-item">
          <span className="hero-perf-label">p99.9</span>
          <span className="hero-perf-value">{fmt(perf?.p999)}</span>
        </div>
      </div>

      <a
        className="hero-github"
        href={githubUrl}
        target="_blank"
        rel="noopener noreferrer"
      >
        GitHub ↗
      </a>
    </header>
  );
};

export default TopBar;
