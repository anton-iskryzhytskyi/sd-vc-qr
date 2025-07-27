# Data Analysis Scripts

Python scripts to analyze benchmark results and create charts.

## Setup

Python 3 is required.  

Create env:
```bash
python3 -m venv env
```

Activate env:
```bash
source env/bin/activate
```

```bash
cd data-analysis &&
pip install -r requirements.txt
```

## Usage

1. Run benchmarks from project root:
   ```bash
   cargo bench
   ```

2. Extract data:
   ```bash
   cd data-analysis
   python export_raw_data.py
   ```

3. Generate charts:
   ```bash
   python visualize_benchmarks.py
   ```

4. Get efficiency rankings (optional):
   ```bash
   python evaluate_efficiency.py --per-scenario
   ```

## Scripts

- export_raw_data.py - Converts Criterion output to CSV files
- visualize_benchmarks.py - Creates comparison charts and analysis
- evaluate_efficiency.py - Calculates efficiency scores
- stats.py - Legacy script, use others instead

## Output

- Data files: ../raw-data/benchmark_data/
- Charts: ../results/plots/
- Summary report: ../results/plots/benchmark_summary_report.txt