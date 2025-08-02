
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
from pathlib import Path
import argparse
import sys

ALGO_LABELS = {
    'Ed25519': 'Ed25519',
    'Secp256k1': 'secp256k1',
    'P256': 'P-256',
    'Dilithium2': 'Dilithium-2',
    'Falcon512': 'Falcon-512',
    'SPHINCS+128s': 'SPHINCS+-128s'
}

COLORS = {
    'Ed25519': '#1b9e77',
    'Secp256k1': '#d95f02',
    'P256': '#7570b3',
    'Dilithium2': '#e7298a',
    'Falcon512': '#66a61e',
    'SPHINCS+128s': '#e6ab02'
}

CLASSICAL_ALGOS = ['Ed25519', 'Secp256k1', 'P256']
QUANTUM_ALGOS = ['Dilithium2', 'Falcon512', 'SPHINCS+128s']


plt.rcParams.update({
    'figure.dpi': 300,
    'savefig.dpi': 300,
    'font.size': 10,
    'axes.titlesize': 12,
    'axes.labelsize': 10,
    'legend.fontsize': 8
})
sns.set_style('whitegrid', {'axes.edgecolor': '.8'})

def _fmt_speed(val):
    return f'{val:.2f} ms' if val >= 1 else f'{val*1000:.0f} µs'

def _fmt_size(size_bytes):
    if size_bytes < 1024:
        return f'{size_bytes} B'
    elif size_bytes < 1024**2:
        return f'{size_bytes/1024:.1f} KB'
    else:
        return f'{size_bytes/(1024**2):.1f} MB'

def load_benchmark_data(data_dir='raw-data'):
    data_dir = Path(data_dir)
    
    try:
        combined_df = pd.read_csv(data_dir / 'combined_summary.csv')
        timing_df = pd.read_csv(data_dir / 'timing_summary.csv')
        samples_df = pd.read_csv(data_dir / 'raw_timing_samples.csv')
        size_df = pd.read_csv(data_dir / 'size_data.csv') if (data_dir / 'size_data.csv').exists() else None
        
        print(f"Loaded data:")
        print(f"  - Combined summary: {len(combined_df)} rows")
        print(f"  - Timing summary: {len(timing_df)} rows")
        print(f"  - Raw samples: {len(samples_df)} rows")
        print(f"  - Size data: {len(size_df) if size_df is not None else 0} rows")
        
        return combined_df, timing_df, samples_df, size_df
    
    except FileNotFoundError as e:
        print(f"Error: Could not find benchmark data files in {data_dir}")
        print(f"Please run 'python export_raw_data.py' first to generate the data files")
        sys.exit(1)

def create_algorithm_comparison(combined_df, output_dir):
    fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(16, 12))
    fig.suptitle('Algorithm Performance Comparison (Classical vs Post-Quantum)', fontsize=16, fontweight='bold')
    
    issue_data = combined_df[combined_df['operation'] == 'issue']
    verify_data = combined_df[combined_df['operation'] == 'verify']
    
    issue_medians = issue_data.groupby('wallet_type')['median_time_ms'].median().sort_values()
    verify_medians = verify_data.groupby('wallet_type')['median_time_ms'].median().sort_values()
    
    # TODO: order?
    algo_order = [algo for algo in CLASSICAL_ALGOS if algo in issue_medians.index] + \
                 [algo for algo in QUANTUM_ALGOS if algo in issue_medians.index]
    
    issue_sorted = issue_medians.reindex(algo_order)
    verify_sorted = verify_medians.reindex(algo_order)
    
    ax1.bar(range(len(issue_sorted)), issue_sorted.values, 
                    color=[COLORS[algo] for algo in issue_sorted.index])
    ax1.set_yscale('log') # as SPHINCS+-128s is very slow compared to others...
    ax1.set_ylabel('Median Issuance Time (ms)')
    ax1.set_title('SD-JWT VC Issuance Performance')
    ax1.set_xticks(range(len(issue_sorted)))
    ax1.set_xticklabels([ALGO_LABELS[algo] for algo in issue_sorted.index], rotation=45)
    ax1.grid(True, alpha=0.3, which='both')
    
    for i, (algo, val) in enumerate(issue_sorted.items()):
        ax1.text(i, val * 1.1, _fmt_speed(val), ha='center', va='bottom', fontsize=8)
    
    ax1.axvline(x=len(CLASSICAL_ALGOS)-0.5, color='gray', linestyle='--', alpha=0.5)
    ax1.text(len(CLASSICAL_ALGOS)/2-0.5, ax1.get_ylim()[1]*0.8, 'Classical', ha='center', 
             bbox=dict(boxstyle='round,pad=0.3', facecolor='lightblue', alpha=0.5))
    ax1.text(len(CLASSICAL_ALGOS) + len(QUANTUM_ALGOS)/2-0.5, ax1.get_ylim()[1]*0.8, 'Post-Quantum', ha='center',
             bbox=dict(boxstyle='round,pad=0.3', facecolor='lightcoral', alpha=0.5))
    
    ax2.bar(range(len(verify_sorted)), verify_sorted.values,
                    color=[COLORS[algo] for algo in verify_sorted.index])
    ax2.set_yscale('log') # as above ^^
    ax2.set_ylabel('Median Verification Time (ms)')
    ax2.set_title('SD-JWT VC Verification Performance')
    ax2.set_xticks(range(len(verify_sorted)))
    ax2.set_xticklabels([ALGO_LABELS[algo] for algo in verify_sorted.index], rotation=45)
    ax2.grid(True, alpha=0.3, which='both')
    
    for i, (algo, val) in enumerate(verify_sorted.items()):
        ax2.text(i, val * 1.1, _fmt_speed(val), ha='center', va='bottom', fontsize=8)
    
    ax2.axvline(x=len(CLASSICAL_ALGOS)-0.5, color='gray', linestyle='--', alpha=0.5)
    
    ratio_data = []
    for algo in algo_order:
        if algo in issue_sorted.index and algo in verify_sorted.index:
            ratio = issue_sorted[algo] / verify_sorted[algo]
            ratio_data.append((algo, ratio))
    
    algos, ratios = zip(*ratio_data)
    ax3.bar(range(len(ratios)), ratios, color=[COLORS[algo] for algo in algos])
    ax3.set_ylabel('Issue/Verify Ratio')
    ax3.set_title('Issuance vs Verification Ratio')
    ax3.set_xticks(range(len(algos)))
    ax3.set_xticklabels([ALGO_LABELS[algo] for algo in algos], rotation=45)
    ax3.grid(True, alpha=0.3)
    
    for i, ratio in enumerate(ratios):
        ax3.text(i, ratio * 1.05, f'{ratio:.1f}×', ha='center', va='bottom', fontsize=8)
    
    combined_perf = []
    for algo in algo_order:
        if algo in issue_sorted.index and algo in verify_sorted.index:
            geom_mean = np.sqrt(issue_sorted[algo] * verify_sorted[algo])
            combined_perf.append((algo, geom_mean))
    
    algos, perfs = zip(*combined_perf)
    ax4.bar(range(len(perfs)), perfs, color=[COLORS[algo] for algo in algos])
    ax4.set_yscale('log') # the same reason as above
    ax4.set_ylabel('Combined Performance (ms)')
    ax4.set_title('Overall Performance (Geometric Mean)')
    ax4.set_xticks(range(len(algos)))
    ax4.set_xticklabels([ALGO_LABELS[algo] for algo in algos], rotation=45)
    ax4.grid(True, alpha=0.3, which='both')
    
    for i, perf in enumerate(perfs):
        ax4.text(i, perf * 1.1, _fmt_speed(perf), ha='center', va='bottom', fontsize=8)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'algorithm_comparison.png', dpi=300, bbox_inches='tight')
    plt.close()

def create_scalability_analysis(combined_df, output_dir):
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(16, 6))
    fig.suptitle('Scalability Analysis: Performance vs Credential Complexity', fontsize=14, fontweight='bold')
    
    
    for operation, ax in [('issue', ax1), ('verify', ax2)]:
        op_data = combined_df[combined_df['operation'] == operation]
        
        for algo in ALGO_LABELS.keys():
            if algo in op_data['wallet_type'].values:
                algo_data = op_data[op_data['wallet_type'] == algo]
                
                scalability = algo_data.groupby('field_count')['median_time_ms'].median()
                
                ax.plot(scalability.index, scalability.values, 
                       marker='o', linewidth=2, markersize=6,
                       color=COLORS[algo], label=ALGO_LABELS[algo])
        
        ax.set_xlabel('Number of Fields in Credential')
        ax.set_ylabel('Median Time (ms)')
        ax.set_title(f'{operation.title()} Performance Scalability')
        ax.set_yscale('log')
        ax.grid(True, alpha=0.3, which='both')
        ax.legend(bbox_to_anchor=(1.02, 1), loc='upper left', frameon=False)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'scalability_analysis.png', dpi=300, bbox_inches='tight')
    plt.close()

def create_heatmaps(combined_df, output_dir):
    
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(18, 8))
    fig.suptitle('Comprehensive Performance Analysis (All Configurations)', fontsize=16, fontweight='bold')
    
    for operation, ax in [('issue', ax1), ('verify', ax2)]:
        op_data = combined_df[combined_df['operation'] == operation]
        
        heatmap_data = op_data.pivot_table(
            values='median_time_ms',
            index='wallet_type',
            columns=['field_count', 'field_size'],
            aggfunc='median'
        )
        
        algo_order = [algo for algo in CLASSICAL_ALGOS if algo in heatmap_data.index] + \
                     [algo for algo in QUANTUM_ALGOS if algo in heatmap_data.index]
        heatmap_data = heatmap_data.reindex(algo_order)
        
        heatmap_log = np.log10(heatmap_data)
        
        im = ax.imshow(heatmap_log.values, cmap='YlOrRd', aspect='auto')
        
        ax.grid(False)
        ax.set_xticks(np.arange(len(heatmap_data.columns)+1)-.5, minor=True)
        ax.set_yticks(np.arange(len(heatmap_data.index)+1)-.5, minor=True)
        ax.grid(which="minor", color="white", linestyle='-', linewidth=2)
        ax.tick_params(which="minor", size=0)
        
        ax.set_yticks(range(len(heatmap_data.index)))
        ax.set_yticklabels([ALGO_LABELS[algo] for algo in heatmap_data.index])
        ax.set_xticks(range(len(heatmap_data.columns)))
        
        col_labels = [f'{fc}f/{fs}B' for fc, fs in heatmap_data.columns]
        ax.set_xticklabels(col_labels, rotation=45, ha='right')
        
        ax.set_title(f'{operation.title()} Performance')
        ax.set_xlabel('Configuration (fields/field_size)')
        
        for i in range(len(heatmap_data.index)):
            for j in range(len(heatmap_data.columns)):
                value = heatmap_data.iloc[i, j]
                if not np.isnan(value):
                    text_color = 'white' if heatmap_log.iloc[i, j] > heatmap_log.values.mean() else 'black'
                    ax.text(j, i, _fmt_speed(value), ha='center', va='center', 
                           fontsize=7, color=text_color, weight='bold')
        
        cbar = plt.colorbar(im, ax=ax, shrink=0.8)
        cbar.set_label('log₁₀(Time in ms)', rotation=270, labelpad=20)
        
        field_counts = sorted(combined_df['field_count'].unique())
        field_sizes = sorted(combined_df['field_size'].unique())
        
        for i in range(1, len(field_counts)):
            separator_pos = i * len(field_sizes) - 0.5
            ax.axvline(x=separator_pos, color='black', linewidth=1, alpha=0.7)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'comprehensive_heatmap.png', dpi=300, bbox_inches='tight')
    plt.close()

def create_size_heatmaps(combined_df, output_dir):
    
    fig, ax2 = plt.subplots(1, 1, figsize=(12, 8))
    # consider putting both, full jwt (with disclosure) and only jwt (w/o)
    #    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(20, 8))
    fig.suptitle('JWT Credential Size Analysis', fontsize=16, fontweight='bold')
    
    jwt_size_data = combined_df.pivot_table(
        values='jwt_size_bytes',
        index='wallet_type',
        columns=['field_count', 'field_size'],
        aggfunc='first'
    )
    
    total_size_data = combined_df.pivot_table(
        values='total_size_bytes',
        index='wallet_type',
        columns=['field_count', 'field_size'],
        aggfunc='first'
    )
    
    algo_order = [algo for algo in CLASSICAL_ALGOS if algo in jwt_size_data.index] + \
                 [algo for algo in QUANTUM_ALGOS if algo in jwt_size_data.index]
    jwt_size_data = jwt_size_data.reindex(algo_order)
    total_size_data = total_size_data.reindex(algo_order)
    
    for ax, size_data, title in [
        # (ax1, jwt_size_data, 'JWT Credential Size (excluding disclosures)'),
        (ax2, total_size_data, 'Total Size (JWT + disclosures)')
    ]:
        size_log = np.log10(size_data)
        
        im = ax.imshow(size_log.values, cmap='Blues', aspect='auto')
        
        ax.grid(False)
        ax.set_xticks(np.arange(len(size_data.columns)+1)-.5, minor=True)
        ax.set_yticks(np.arange(len(size_data.index)+1)-.5, minor=True)
        ax.grid(which="minor", color="white", linestyle='-', linewidth=2)
        ax.tick_params(which="minor", size=0)
        
        ax.set_yticks(range(len(size_data.index)))
        ax.set_yticklabels([ALGO_LABELS[algo] for algo in size_data.index])
        ax.set_xticks(range(len(size_data.columns)))
        
        col_labels = [f'{fc}f/{fs}B' for fc, fs in size_data.columns]
        ax.set_xticklabels(col_labels, rotation=45, ha='right')
        
        ax.set_title(title)
        ax.set_xlabel('Configuration (fields/field_size)')
        
        for i in range(len(size_data.index)):
            for j in range(len(size_data.columns)):
                value = size_data.iloc[i, j]
                if not np.isnan(value):
                    text_color = 'white' if size_log.iloc[i, j] > size_log.values.mean() else 'black'
                    ax.text(j, i, _fmt_size(int(value)), ha='center', va='center', 
                           fontsize=7, color=text_color, weight='bold')

        cbar = plt.colorbar(im, ax=ax, shrink=0.8)
        cbar.set_label('log₁₀(Size in bytes)', rotation=270, labelpad=20)
        
        field_counts = sorted(combined_df['field_count'].unique())
        field_sizes = sorted(combined_df['field_size'].unique())
        
        for i in range(1, len(field_counts)):
            separator_pos = i * len(field_sizes) - 0.5
            ax.axvline(x=separator_pos, color='black', linewidth=1, alpha=0.7)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'size_heatmaps.png', dpi=300, bbox_inches='tight')
    plt.close()

def create_distribution_analysis(samples_df, output_dir):
    fig, axes = plt.subplots(2, 3, figsize=(20, 12))
    fig.suptitle('Latency Distribution Analysis - Stability and Variance by Algorithm', fontsize=16, fontweight='bold')
    
    operations = ['issue', 'verify']
    algo_groups = [CLASSICAL_ALGOS, QUANTUM_ALGOS]
    group_names = ['Classical Algorithms', 'Post-Quantum Algorithms']
    
    for op_idx, operation in enumerate(operations):
        op_data = samples_df[samples_df['operation'] == operation]
        
        for group_idx, (algos, group_name) in enumerate(zip(algo_groups, group_names)):
            ax = axes[op_idx, group_idx]
            
            algo_data = []
            algo_names = []
            
            for algo in algos:
                if algo in op_data['wallet_type'].values:
                    algo_samples = op_data[op_data['wallet_type'] == algo]['per_op_time_ms']
                    if len(algo_samples) > 0:
                        algo_data.append(algo_samples.values)
                        algo_names.append(ALGO_LABELS[algo])
            
            if algo_data:
                parts = ax.violinplot(algo_data, positions=range(len(algo_data)), 
                                    showmeans=True, showmedians=True, showextrema=True)
                
                for i, (pc, algo_name) in enumerate(zip(parts['bodies'], algo_names)):
                    algo_key = [k for k, v in ALGO_LABELS.items() if v == algo_name][0]
                    pc.set_facecolor(COLORS[algo_key])
                    pc.set_alpha(0.7)
                
                parts['cmeans'].set_color('red')
                parts['cmedians'].set_color('blue')
                parts['cbars'].set_color('black')
                parts['cmins'].set_color('black')
                parts['cmaxes'].set_color('black')
                
                ax.set_xticks(range(len(algo_names)))
                ax.set_xticklabels(algo_names, rotation=45, ha='right')
                ax.set_ylabel('Latency (ms)')
                ax.set_title(f'{operation.title()} - {group_name}')
                ax.set_yscale('log')
                ax.grid(True, alpha=0.3)
                
                if op_idx == 0 and group_idx == 0:
                    ax.plot([], [], 'r-', label='Mean', linewidth=2)
                    ax.plot([], [], 'b-', label='Median', linewidth=2)
                    ax.legend(loc='upper right')
            else:
                ax.text(0.5, 0.5, f'No data for {group_name}', 
                       ha='center', va='center', transform=ax.transAxes)
                ax.set_title(f'{operation.title()} - {group_name}')
        

        ax_stats = axes[op_idx, 2]
        
        stability_data = []
        labels = []
        colors = []
        
        for algo in ALGO_LABELS.keys():
            if algo in op_data['wallet_type'].values:
                algo_samples = op_data[op_data['wallet_type'] == algo]['per_op_time_ms']
                if len(algo_samples) > 0:
                    cv = algo_samples.std() / algo_samples.mean() * 100  # Coefficient of variation as percentage
                    stability_data.append(cv)
                    labels.append(ALGO_LABELS[algo])
                    colors.append(COLORS[algo])
        
        if stability_data:
            bars = ax_stats.bar(range(len(stability_data)), stability_data, color=colors, alpha=0.7)
            ax_stats.set_xticks(range(len(labels)))
            ax_stats.set_xticklabels(labels, rotation=45, ha='right')
            ax_stats.set_ylabel('Coefficient of Variation (%)')
            ax_stats.set_title(f'{operation.title()} - Stability (CV%)')
            ax_stats.grid(True, alpha=0.3)

            for bar, cv in zip(bars, stability_data):
                height = bar.get_height()
                ax_stats.text(bar.get_x() + bar.get_width()/2., height + height*0.01,
                            f'{cv:.1f}%', ha='center', va='bottom', fontsize=8)
        else:
            ax_stats.text(0.5, 0.5, f'No data for {operation.title()}', 
                         ha='center', va='center', transform=ax_stats.transAxes)
            ax_stats.set_title(f'{operation.title()} - Stability (CV%)')
    
    plt.tight_layout()
    plt.savefig(output_dir / 'distribution_analysis.png', dpi=300, bbox_inches='tight')
    plt.close()

def create_detailed_variance_analysis(samples_df, combined_df, output_dir):
    fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(16, 12))
    fig.suptitle('Detailed Variance and Stability Analysis', fontsize=16, fontweight='bold')
    
    operations = ['issue', 'verify']
    
    for op_idx, operation in enumerate(operations):
        ax = [ax1, ax2][op_idx]
        op_data = samples_df[samples_df['operation'] == operation]
        
        algo_data = []
        algo_names = []

        medians = []
        for algo in ALGO_LABELS.keys():
            if algo in op_data['wallet_type'].values:
                algo_samples = op_data[op_data['wallet_type'] == algo]['per_op_time_ms']
                if len(algo_samples) > 0:
                    medians.append((algo_samples.median(), algo))
        
        medians.sort()
        
        for _, algo in medians:
            algo_samples = op_data[op_data['wallet_type'] == algo]['per_op_time_ms']
            algo_data.append(algo_samples.values)
            algo_names.append(ALGO_LABELS[algo])
        
        if algo_data:
            bp = ax.boxplot(algo_data, labels=algo_names, patch_artist=True, 
                           showfliers=True, flierprops=dict(marker='o', markersize=3, alpha=0.5))
            
            for patch, algo_name in zip(bp['boxes'], algo_names):
                algo_key = [k for k, v in ALGO_LABELS.items() if v == algo_name][0]
                patch.set_facecolor(COLORS[algo_key])
                patch.set_alpha(0.7)
            
            ax.set_ylabel('Latency (ms)')
            ax.set_title(f'{operation.title()} Performance Distribution')
            ax.set_yscale('log')
            ax.grid(True, alpha=0.3)

            plt.setp(ax.get_xticklabels(), rotation=45, ha='right')

    ax = ax3
    
    variance_data = combined_df.groupby(['wallet_type', 'operation']).agg({
        'std_dev_time_ms': 'mean',
        'mean_time_ms': 'mean'
    }).reset_index()
    
    variance_data['cv_percent'] = (variance_data['std_dev_time_ms'] / variance_data['mean_time_ms']) * 100

    algos = list(ALGO_LABELS.keys())
    x_pos = np.arange(len(algos))
    width = 0.35
    
    issue_cv = []
    verify_cv = []
    
    for algo in algos:
        issue_data = variance_data[(variance_data['wallet_type'] == algo) & (variance_data['operation'] == 'issue')]
        verify_data = variance_data[(variance_data['wallet_type'] == algo) & (variance_data['operation'] == 'verify')]
        
        issue_cv.append(issue_data['cv_percent'].iloc[0] if len(issue_data) > 0 else 0)
        verify_cv.append(verify_data['cv_percent'].iloc[0] if len(verify_data) > 0 else 0)
    
    bars1 = ax.bar(x_pos - width/2, issue_cv, width, label='Issue', alpha=0.8)
    bars2 = ax.bar(x_pos + width/2, verify_cv, width, label='Verify', alpha=0.8)

    for i, (bar1, bar2, algo) in enumerate(zip(bars1, bars2, algos)):
        bar1.set_color(COLORS[algo])
        bar2.set_color(COLORS[algo])
        bar2.set_alpha(0.6)  # Make verify bars slightly transparent to show diff
    
    ax.set_xlabel('Algorithm')
    ax.set_ylabel('Coefficient of Variation (%)')
    ax.set_title('Performance Consistency Comparison')
    ax.set_xticks(x_pos)
    ax.set_xticklabels([ALGO_LABELS[algo] for algo in algos], rotation=45, ha='right')
    ax.legend()
    ax.grid(True, alpha=0.3)

    ax = ax4
    
    bars1 = ax.bar(x_pos - width/2, [variance_data[(variance_data['wallet_type'] == algo) & (variance_data['operation'] == 'issue')]['std_dev_time_ms'].iloc[0] 
                                    if len(variance_data[(variance_data['wallet_type'] == algo) & (variance_data['operation'] == 'issue')]) > 0 else 0 
                                    for algo in algos], 
                  width, label='Issue', alpha=0.8)
    bars2 = ax.bar(x_pos + width/2, [variance_data[(variance_data['wallet_type'] == algo) & (variance_data['operation'] == 'verify')]['std_dev_time_ms'].iloc[0] 
                                    if len(variance_data[(variance_data['wallet_type'] == algo) & (variance_data['operation'] == 'verify')]) > 0 else 0 
                                    for algo in algos], 
                  width, label='Verify', alpha=0.8)

    for i, (bar1, bar2, algo) in enumerate(zip(bars1, bars2, algos)):
        bar1.set_color(COLORS[algo])
        bar2.set_color(COLORS[algo])
        bar2.set_alpha(0.6)
    
    ax.set_xlabel('Algorithm')
    ax.set_ylabel('Standard Deviation (ms)')
    ax.set_title('Absolute Performance Variance')
    ax.set_xticks(x_pos)
    ax.set_xticklabels([ALGO_LABELS[algo] for algo in algos], rotation=45, ha='right')
    ax.set_yscale('log')
    ax.legend()
    ax.grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(output_dir / 'variance_analysis.png', dpi=300, bbox_inches='tight')
    plt.close()

def generate_summary_report(combined_df, size_df, output_dir):
    report_path = output_dir / 'benchmark_summary_report.txt'
    
    with open(report_path, 'w') as f:
        f.write("SD-JWT VC Benchmark Analysis - Summary Report\n")
        f.write("=" * 50 + "\n\n")
        
        f.write("Performance Rankings (lowest - highest across configurations):\n")
        f.write("-" * 60 + "\n")
        
        for operation in ['issue', 'verify']:
            op_data = combined_df[combined_df['operation'] == operation]
            
            perf_stats = op_data.groupby('wallet_type')['median_time_ms'].agg(['min', 'max']).sort_values('min')
            
            f.write(f"\n{operation.title()} Performance:\n")
            for i, (algo, stats) in enumerate(perf_stats.iterrows(), 1):
                category = "Classical" if algo in CLASSICAL_ALGOS else "Post-Quantum"
                min_time = stats['min']
                max_time = stats['max']
                f.write(f"  {i}. {ALGO_LABELS[algo]:15} {_fmt_speed(min_time):>10} - {_fmt_speed(max_time):>10} ({category})\n")
        
        if size_df is not None:
            f.write(f"\nCredential Size Analysis:\n")
            f.write("-" * 40 + "\n")
            size_ranking = combined_df.groupby('wallet_type')['jwt_size_bytes'].median().sort_values()
            
            for i, (algo, size_bytes) in enumerate(size_ranking.items(), 1):
                category = "Classical" if algo in CLASSICAL_ALGOS else "Post-Quantum"
                f.write(f"  {i}. {ALGO_LABELS[algo]:15} {_fmt_size(int(size_bytes)):>10} ({category})\n")
        
        f.write(f"\nPerformance Consistency (Coefficient of Variation):\n")
        f.write("-" * 40 + "\n")
        
        for operation in ['issue', 'verify']:
            op_data = combined_df[combined_df['operation'] == operation]
            cv_data = op_data.groupby('wallet_type').apply(
                lambda x: x['std_dev_time_ms'].mean() / x['mean_time_ms'].mean(), include_groups=False
            ).sort_values()
            
            f.write(f"\n{operation.title()} Consistency (CV - lower is better):\n")
            for i, (algo, cv) in enumerate(cv_data.items(), 1):
                category = "Classical" if algo in CLASSICAL_ALGOS else "Post-Quantum"
                f.write(f"  {i}. {ALGO_LABELS[algo]:15} {cv:>6.3f} ({category})\n")
    
    print(f"Summary report generated: {report_path}")

def main():
    parser = argparse.ArgumentParser(description='Generate SD-JWT VC benchmark visualizations')
    parser.add_argument('--data-dir', default='../target/criterion')
    parser.add_argument('--output-dir', default='../results')
    
    args = parser.parse_args()
    
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)
    
    print("SD-JWT VC Benchmark Visualization")
    print("=" * 40)
    
    combined_df, _, samples_df, size_df = load_benchmark_data(args.data_dir)
    
    print("\nGenerating visualizations...")
    
    print("  1. Algorithm comparison...")
    create_algorithm_comparison(combined_df, output_dir)
    
    print("  2. Scalability analysis...")
    create_scalability_analysis(combined_df, output_dir)
    
    print("  3. Performance heatmaps...")
    create_heatmaps(combined_df, output_dir)
    
    print("  4. Size heatmaps...")
    create_size_heatmaps(combined_df, output_dir)
    
    print("  5. Detailed variance analysis...")
    create_detailed_variance_analysis(samples_df, combined_df, output_dir)
    
    print("  6. Summary report...")
    generate_summary_report(combined_df, size_df, output_dir)
    
    print(f"\nAll visualizations generated in {output_dir}/")
    print("\nGenerated files:")
    print("  - algorithm_comparison.png")
    print("  - scalability_analysis.png")
    print("  - comprehensive_heatmap.png")
    print("  - size_heatmaps.png")
    print("  - variance_analysis.png")
    print("  - benchmark_summary_report.txt")

if __name__ == "__main__":
    main()